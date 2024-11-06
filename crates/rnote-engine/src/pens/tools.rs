// Imports
use super::pensconfig::toolsconfig::ToolStyle;
use super::PenBehaviour;
use super::PenStyle;
use crate::engine::{EngineView, EngineViewMut};
use crate::store::StrokeKey;
use crate::{Camera, DrawableOnDoc, WidgetFlags};
use p2d::bounding_volume::Aabb;
use p2d::bounding_volume::BoundingVolume;
use piet::RenderContext;
use rnote_compose::builders::buildable::Buildable;
use rnote_compose::builders::buildable::BuilderCreator;
use rnote_compose::builders::buildable::BuilderProgress;
use rnote_compose::builders::PenPathBuilderType;
use rnote_compose::builders::PenPathCurvedBuilder;
use rnote_compose::builders::PenPathModeledBuilder;
use rnote_compose::builders::PenPathSimpleBuilder;
use rnote_compose::color;
use rnote_compose::eventresult::{EventPropagation, EventResult};
use rnote_compose::ext::{AabbExt, Vector2Ext};
use rnote_compose::penevent::{PenEvent, PenProgress};
use rnote_compose::penpath::Element;
use rnote_compose::penpath::Segment;
use rnote_compose::shapes::Shapeable;
use rnote_compose::Constraints;
use rnote_compose::PenPath;
use std::time::Duration;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct LaserStore {
    pub stroke_paths: Vec<PenPath>,
    pub stroke_update_time: Instant,
}

impl Default for LaserStore {
    fn default() -> Self {
        Self {
            stroke_paths: Vec::new(),
            stroke_update_time: Instant::now(),
        }
    }
}

impl LaserStore {
    pub const FULL_FADE_DURATION: Duration = Duration::from_millis(1500);

    pub fn new_stroke(&mut self, element: Element, now: Instant) {
        if self.is_faded() {
            self.stroke_paths.clear();
        }

        self.stroke_paths.push(PenPath::new(element));
        self.stroke_update_time = now;
    }

    pub fn update(&mut self, progress: BuilderProgress<Segment>, now: Instant) {
        if let Some(last_stroke) = self.stroke_paths.last_mut() {
            match progress {
                BuilderProgress::InProgress => {}
                BuilderProgress::EmitContinue(segments) | BuilderProgress::Finished(segments) => {
                    last_stroke.extend(segments);
                }
            };
        }

        self.stroke_update_time = now;
    }

    pub fn is_faded(&self) -> bool {
        self.stroke_update_time.elapsed() >= Self::FULL_FADE_DURATION
    }
}

#[derive(Default, Debug)]
pub struct LaserTool {
    path_builder: Option<Box<dyn Buildable<Emit = Segment>>>,
}

impl DrawableOnDoc for LaserTool {
    fn bounds_on_doc(&self, engine_view: &EngineView) -> Option<Aabb> {
        if engine_view.store.laser_store.is_faded() {
            return None;
        }

        let strokes = engine_view.store.laser_store.stroke_paths.iter();

        strokes
            .map(|path| path.bounds())
            .reduce(|acc, path| acc.merged(&path))
            .map(|bounds| {
                bounds.extend_by(na::Vector2::repeat(
                    Self::OUTER_STROKE_WIDTH / engine_view.camera.total_zoom(),
                ))
            })
    }

    fn draw_on_doc(
        &self,
        cx: &mut piet_cairo::CairoRenderContext,
        engine_view: &EngineView,
    ) -> anyhow::Result<()> {
        cx.save().map_err(|e| anyhow::anyhow!("{e:?}"))?;

        let transparency = engine_view
            .store
            .laser_store
            .stroke_update_time
            .elapsed()
            .div_duration_f64(Self::FULL_FADE_DURATION)
            .clamp(0.0, 1.0);

        let opacity: u8 = ((1.0 - transparency) * 255.0).round() as u8;

        for pen_path in &engine_view.store.laser_store.stroke_paths {
            let total_zoom = engine_view.camera.total_zoom();
            let bez_path = pen_path.to_kurbo_flattened(0.5);

            cx.stroke_styled(
                &bez_path,
                &Self::OUTER_STROKE_COLOR.with_a8(opacity),
                Self::OUTER_STROKE_WIDTH / total_zoom,
                &LaserTool::STYLE,
            );

            cx.stroke_styled(
                &bez_path,
                &Self::INNER_STROKE_COLOR.with_a8(opacity),
                Self::INNER_STROKE_WIDTH / total_zoom,
                &LaserTool::STYLE,
            );
        }

        cx.restore().map_err(|e| anyhow::anyhow!("{e:?}"))?;
        Ok(())
    }
}

impl LaserTool {
    const FULL_FADE_DURATION: Duration = Duration::from_millis(1500);

    const OUTER_STROKE_WIDTH: f64 = 6.0;
    const INNER_STROKE_WIDTH: f64 = 1.0;

    const INNER_STROKE_COLOR: piet::Color = color::GNOME_BRIGHTS[1];
    const OUTER_STROKE_COLOR: piet::Color = color::GNOME_REDS[1];

    const STYLE: piet::StrokeStyle = piet::StrokeStyle::new()
        .line_join(piet::LineJoin::Round)
        .line_cap(piet::LineCap::Round);
}

#[derive(Clone, Debug)]
pub struct VerticalSpaceTool {
    start_pos_y: f64,
    pos_y: f64,
    limit_x: Option<(f64, f64)>,
    strokes_below: Vec<StrokeKey>,
}

impl Default for VerticalSpaceTool {
    fn default() -> Self {
        Self {
            start_pos_y: 0.0,
            pos_y: 0.0,
            limit_x: None,
            strokes_below: vec![],
        }
    }
}

impl VerticalSpaceTool {
    const Y_OFFSET_THRESHOLD: f64 = 0.1;
    const SNAP_START_POS_DIST: f64 = 10.;
    const OFFSET_LINE_COLOR: piet::Color = color::GNOME_BLUES[3];
    const THRESHOLD_LINE_WIDTH: f64 = 3.0;
    const THRESHOLD_LINE_DASH_PATTERN: [f64; 2] = [9.0, 6.0];
    const OFFSET_LINE_WIDTH: f64 = 1.5;
    const FILL_COLOR: piet::Color = color::GNOME_BRIGHTS[2].with_a8(23);
    const THRESHOLD_LINE_COLOR: piet::Color = color::GNOME_GREENS[4].with_a8(240);
}

impl DrawableOnDoc for VerticalSpaceTool {
    fn bounds_on_doc(&self, engine_view: &EngineView) -> Option<Aabb> {
        let viewport = engine_view.camera.viewport();

        let x = viewport.mins[0];
        let y = self.start_pos_y;
        let width = viewport.extents()[0];
        let height = self.pos_y - self.start_pos_y;
        let tool_bounds = Aabb::new_positive(na::point![x, y], na::point![x + width, y + height]);

        Some(tool_bounds)
    }

    fn draw_on_doc(
        &self,
        cx: &mut piet_cairo::CairoRenderContext,
        engine_view: &EngineView,
    ) -> anyhow::Result<()> {
        cx.save().map_err(|e| anyhow::anyhow!("{e:?}"))?;

        let total_zoom = engine_view.camera.total_zoom();
        let viewport = engine_view.camera.viewport();
        let x = if self.limit_x.is_some() {
            viewport.mins[0].max(self.limit_x.unwrap().0)
        } else {
            viewport.mins[0]
        };
        let y = self.start_pos_y;
        let width = if self.limit_x.is_some() {
            self.limit_x.unwrap().1 - viewport.mins[0].max(self.limit_x.unwrap().0)
        } else {
            viewport.extents()[0]
        };
        let height = self.pos_y - self.start_pos_y;
        let tool_bounds = Aabb::new_positive(na::point![x, y], na::point![x + width, y + height]);

        let tool_bounds_rect = kurbo::Rect::from_points(
            tool_bounds.mins.coords.to_kurbo_point(),
            tool_bounds.maxs.coords.to_kurbo_point(),
        );
        cx.fill(tool_bounds_rect, &Self::FILL_COLOR);

        let threshold_line =
            kurbo::Line::new(kurbo::Point::new(x, y), kurbo::Point::new(x + width, y));
        cx.stroke_styled(
            threshold_line,
            &Self::THRESHOLD_LINE_COLOR,
            Self::THRESHOLD_LINE_WIDTH / total_zoom,
            &piet::StrokeStyle::new().dash_pattern(&Self::THRESHOLD_LINE_DASH_PATTERN),
        );

        let offset_line = kurbo::Line::new(
            kurbo::Point::new(x, y + height),
            kurbo::Point::new(x + width, y + height),
        );
        cx.stroke(
            offset_line,
            &Self::OFFSET_LINE_COLOR,
            Self::OFFSET_LINE_WIDTH / total_zoom,
        );
        cx.restore().map_err(|e| anyhow::anyhow!("{e:?}"))?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct OffsetCameraTool {
    pub start: na::Vector2<f64>,
}

impl Default for OffsetCameraTool {
    fn default() -> Self {
        Self {
            start: na::Vector2::zeros(),
        }
    }
}

impl OffsetCameraTool {
    const CURSOR_SIZE: na::Vector2<f64> = na::vector![16.0, 16.0];
    const CURSOR_STROKE_WIDTH: f64 = 2.0;
    const CURSOR_PATH: &'static str = "m 8 1.078125 l -3 3 h 2 v 2.929687 h -2.960938 v -2 l -3 3 l 3 3 v -2 h 2.960938 v 2.960938 h -2 l 3 3 l 3 -3 h -2 v -2.960938 h 3.054688 v 2 l 3 -3 l -3 -3 v 2 h -3.054688 v -2.929687 h 2 z m 0 0";
    const DARK_COLOR: piet::Color = color::GNOME_DARKS[3].with_a8(240);
    const LIGHT_COLOR: piet::Color = color::GNOME_BRIGHTS[1].with_a8(240);
}

impl DrawableOnDoc for OffsetCameraTool {
    fn bounds_on_doc(&self, engine_view: &EngineView) -> Option<Aabb> {
        Some(Aabb::from_half_extents(
            self.start.into(),
            ((Self::CURSOR_SIZE + na::Vector2::repeat(Self::CURSOR_STROKE_WIDTH)) * 0.5)
                / engine_view.camera.total_zoom(),
        ))
    }

    fn draw_on_doc(
        &self,
        cx: &mut piet_cairo::CairoRenderContext,
        engine_view: &EngineView,
    ) -> anyhow::Result<()> {
        cx.save().map_err(|e| anyhow::anyhow!("{e:?}"))?;

        if let Some(bounds) = self.bounds_on_doc(engine_view) {
            cx.transform(kurbo::Affine::translate(bounds.mins.coords.to_kurbo_vec()));
            cx.transform(kurbo::Affine::scale(1.0 / engine_view.camera.total_zoom()));

            let bez_path = kurbo::BezPath::from_svg(Self::CURSOR_PATH).unwrap();

            cx.stroke(
                bez_path.clone(),
                &Self::LIGHT_COLOR,
                Self::CURSOR_STROKE_WIDTH,
            );
            cx.fill(bez_path, &Self::DARK_COLOR);
        }

        cx.restore().map_err(|e| anyhow::anyhow!("{e:?}"))?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct ZoomTool {
    pub start_surface_coord: na::Vector2<f64>,
    pub current_surface_coord: na::Vector2<f64>,
}

impl Default for ZoomTool {
    fn default() -> Self {
        Self {
            start_surface_coord: na::Vector2::zeros(),
            current_surface_coord: na::Vector2::zeros(),
        }
    }
}

impl ZoomTool {
    const CURSOR_RADIUS: f64 = 4.0;
    const CURSOR_STROKE_WIDTH: f64 = 2.0;
    const DARK_COLOR: piet::Color = color::GNOME_DARKS[3].with_a8(240);
    const LIGHT_COLOR: piet::Color = color::GNOME_BRIGHTS[1].with_a8(240);
}

impl DrawableOnDoc for ZoomTool {
    fn bounds_on_doc(&self, engine_view: &EngineView) -> Option<Aabb> {
        let start_circle_center = engine_view
            .camera
            .transform()
            .inverse()
            .transform_point(&self.start_surface_coord.into());
        let current_circle_center = engine_view
            .camera
            .transform()
            .inverse()
            .transform_point(&self.current_surface_coord.into());

        Some(
            Aabb::new_positive(start_circle_center, current_circle_center).extend_by(
                na::Vector2::repeat(Self::CURSOR_RADIUS + Self::CURSOR_STROKE_WIDTH * 0.5)
                    / engine_view.camera.total_zoom(),
            ),
        )
    }

    fn draw_on_doc(
        &self,
        cx: &mut piet_cairo::CairoRenderContext,
        engine_view: &EngineView,
    ) -> anyhow::Result<()> {
        cx.save().map_err(|e| anyhow::anyhow!("{e:?}"))?;
        let total_zoom = engine_view.camera.total_zoom();

        let start_circle_center = engine_view
            .camera
            .transform()
            .inverse()
            .transform_point(&self.start_surface_coord.into())
            .coords
            .to_kurbo_point();
        let current_circle_center = engine_view
            .camera
            .transform()
            .inverse()
            .transform_point(&self.current_surface_coord.into())
            .coords
            .to_kurbo_point();

        // start circle
        cx.fill(
            kurbo::Circle::new(start_circle_center, Self::CURSOR_RADIUS * 0.8 / total_zoom),
            &Self::LIGHT_COLOR,
        );
        cx.fill(
            kurbo::Circle::new(start_circle_center, Self::CURSOR_RADIUS * 0.6 / total_zoom),
            &Self::DARK_COLOR,
        );

        // current circle
        cx.stroke(
            kurbo::Circle::new(current_circle_center, Self::CURSOR_RADIUS / total_zoom),
            &Self::LIGHT_COLOR,
            Self::CURSOR_STROKE_WIDTH / total_zoom,
        );
        cx.stroke(
            kurbo::Circle::new(current_circle_center, Self::CURSOR_RADIUS / total_zoom),
            &Self::DARK_COLOR,
            Self::CURSOR_STROKE_WIDTH * 0.7 / total_zoom,
        );

        cx.restore().map_err(|e| anyhow::anyhow!("{e:?}"))?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
enum ToolsState {
    Idle,
    Active,
}

impl Default for ToolsState {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Debug, Default)]
pub struct Tools {
    pub verticalspace_tool: VerticalSpaceTool,
    pub offsetcamera_tool: OffsetCameraTool,
    pub zoom_tool: ZoomTool,
    pub laser_tool: LaserTool,
    state: ToolsState,
}

impl PenBehaviour for Tools {
    fn init(&mut self, _engine_view: &EngineView) -> WidgetFlags {
        WidgetFlags::default()
    }

    fn deinit(&mut self) -> WidgetFlags {
        WidgetFlags::default()
    }

    fn style(&self) -> PenStyle {
        PenStyle::Tools
    }

    fn update_state(&mut self, _engine_view: &mut EngineViewMut) -> WidgetFlags {
        WidgetFlags::default()
    }

    fn handle_event(
        &mut self,
        event: PenEvent,
        now: Instant,
        engine_view: &mut EngineViewMut,
    ) -> (EventResult<PenProgress>, WidgetFlags) {
        let mut widget_flags = WidgetFlags::default();

        let event_result = match (&mut self.state, &event) {
            (ToolsState::Idle, PenEvent::Down { element, .. }) => {
                match engine_view.pens_config.tools_config.style {
                    ToolStyle::VerticalSpace => {
                        self.verticalspace_tool.start_pos_y = element.pos[1];
                        self.verticalspace_tool.pos_y = element.pos[1];

                        let pos_x = element.pos[0];

                        let limit_movement_horizontal_borders = engine_view
                            .pens_config
                            .tools_config
                            .verticalspace_tool_config
                            .limit_movement_horizontal_borders;
                        let limit_movement_vertical_borders = engine_view
                            .pens_config
                            .tools_config
                            .verticalspace_tool_config
                            .limit_movement_vertical_borders;

                        let y_max = ((self.verticalspace_tool.pos_y
                            / engine_view.document.format.height())
                        .floor()
                            + 1.0f64)
                            * engine_view.document.format.height();

                        let limit_x = {
                            let page_number_hor =
                                (pos_x / engine_view.document.format.width()).floor();
                            (
                                page_number_hor * engine_view.document.format.width(),
                                (page_number_hor + 1.0f64) * engine_view.document.format.width(),
                            )
                        };

                        self.verticalspace_tool.limit_x = if limit_movement_vertical_borders {
                            Some(limit_x)
                        } else {
                            None
                        };

                        self.verticalspace_tool.strokes_below = engine_view.store.keys_between(
                            self.verticalspace_tool.pos_y,
                            y_max,
                            limit_x,
                            limit_movement_vertical_borders,
                            limit_movement_horizontal_borders,
                        );
                    }
                    ToolStyle::OffsetCamera => {
                        self.offsetcamera_tool.start = element.pos;
                    }
                    ToolStyle::Zoom => {
                        self.zoom_tool.start_surface_coord = engine_view
                            .camera
                            .transform()
                            .transform_point(&element.pos.into())
                            .coords;
                        self.zoom_tool.current_surface_coord = engine_view
                            .camera
                            .transform()
                            .transform_point(&element.pos.into())
                            .coords;
                    }
                    ToolStyle::Laser => {
                        engine_view.store.laser_store.new_stroke(*element, now);

                        self.laser_tool.path_builder =
                            Some(match engine_view.pens_config.brush_config.builder_type {
                                PenPathBuilderType::Simple => {
                                    Box::new(PenPathSimpleBuilder::start(*element, now))
                                }
                                PenPathBuilderType::Curved => {
                                    Box::new(PenPathCurvedBuilder::start(*element, now))
                                }
                                PenPathBuilderType::Modeled => {
                                    Box::new(PenPathModeledBuilder::start(*element, now))
                                }
                            });
                    }
                }
                widget_flags |= engine_view
                    .document
                    .resize_autoexpand(engine_view.store, engine_view.camera);

                self.state = ToolsState::Active;

                EventResult {
                    handled: true,
                    propagate: EventPropagation::Stop,
                    progress: PenProgress::InProgress,
                }
            }
            (ToolsState::Idle, _) => EventResult {
                handled: false,
                propagate: EventPropagation::Proceed,
                progress: PenProgress::Idle,
            },
            (ToolsState::Active, PenEvent::Down { element, .. }) => {
                match engine_view.pens_config.tools_config.style {
                    ToolStyle::VerticalSpace => {
                        let y_offset = if (element.pos[1] - self.verticalspace_tool.start_pos_y)
                            .abs()
                            < VerticalSpaceTool::SNAP_START_POS_DIST
                        {
                            self.verticalspace_tool.start_pos_y - self.verticalspace_tool.pos_y
                        } else {
                            engine_view.document.snap_position(
                                element.pos - na::vector![0., self.verticalspace_tool.pos_y],
                            )[1]
                        };

                        if y_offset.abs() > VerticalSpaceTool::Y_OFFSET_THRESHOLD {
                            engine_view.store.translate_strokes(
                                &self.verticalspace_tool.strokes_below,
                                na::vector![0.0, y_offset],
                            );
                            engine_view.store.translate_strokes_images(
                                &self.verticalspace_tool.strokes_below,
                                na::vector![0.0, y_offset],
                            );
                            self.verticalspace_tool.pos_y += y_offset;

                            widget_flags.store_modified = true;
                        }

                        // possibly nudge camera
                        widget_flags |= engine_view
                            .camera
                            .nudge_w_pos(element.pos, engine_view.document);
                        widget_flags |= engine_view
                            .document
                            .expand_autoexpand(engine_view.camera, engine_view.store);
                        engine_view.store.regenerate_rendering_in_viewport_threaded(
                            engine_view.tasks_tx.clone(),
                            false,
                            engine_view.camera.viewport(),
                            engine_view.camera.image_scale(),
                        );
                    }
                    ToolStyle::OffsetCamera => {
                        let offset = engine_view
                            .camera
                            .transform()
                            .transform_point(&element.pos.into())
                            .coords
                            - engine_view
                                .camera
                                .transform()
                                .transform_point(&self.offsetcamera_tool.start.into())
                                .coords;

                        widget_flags |= engine_view
                            .camera
                            .set_offset(engine_view.camera.offset() - offset, engine_view.document);
                        widget_flags |= engine_view
                            .document
                            .resize_autoexpand(engine_view.store, engine_view.camera);
                    }
                    ToolStyle::Zoom => {
                        let total_zoom_old = engine_view.camera.total_zoom();
                        let camera_offset = engine_view.camera.offset();

                        let new_surface_coord = engine_view
                            .camera
                            .transform()
                            .transform_point(&element.pos.into())
                            .coords;

                        let offset = new_surface_coord - self.zoom_tool.current_surface_coord;

                        // Drag down zooms out, drag up zooms in
                        let new_zoom =
                            total_zoom_old * (1.0 - offset[1] * Camera::DRAG_ZOOM_MAGN_ZOOM_FACTOR);

                        if (Camera::ZOOM_MIN..=Camera::ZOOM_MAX).contains(&new_zoom) {
                            widget_flags |= engine_view
                                .camera
                                .zoom_w_timeout(new_zoom, engine_view.tasks_tx.clone());

                            // Translate the camera view so that the start_surface_coord has the same surface position
                            // as before the zoom occurred
                            let new_camera_offset = (((camera_offset
                                + self.zoom_tool.start_surface_coord)
                                / total_zoom_old)
                                * new_zoom)
                                - self.zoom_tool.start_surface_coord;
                            widget_flags |= engine_view
                                .camera
                                .set_offset(new_camera_offset, engine_view.document);

                            widget_flags |= engine_view
                                .document
                                .expand_autoexpand(engine_view.camera, engine_view.store);
                        }
                        self.zoom_tool.current_surface_coord = new_surface_coord;
                    }
                    ToolStyle::Laser => {
                        if let Some(builder) = &mut self.laser_tool.path_builder {
                            let builder_result =
                                builder.handle_event(event, now, Constraints::default());

                            engine_view
                                .store
                                .laser_store
                                .update(builder_result.progress, now);
                        }
                    }
                }

                EventResult {
                    handled: true,
                    propagate: EventPropagation::Stop,
                    progress: PenProgress::InProgress,
                }
            }
            (ToolsState::Active, PenEvent::Up { .. }) => {
                match engine_view.pens_config.tools_config.style {
                    ToolStyle::VerticalSpace => {
                        engine_view
                            .store
                            .update_geometry_for_strokes(&self.verticalspace_tool.strokes_below);

                        widget_flags |= engine_view.store.record(Instant::now());
                        widget_flags.store_modified = true;
                    }
                    ToolStyle::Laser => {
                        if let Some(builder) = &mut self.laser_tool.path_builder {
                            let builder_result =
                                builder.handle_event(event, now, Constraints::default());

                            engine_view
                                .store
                                .laser_store
                                .update(builder_result.progress, now);

                            engine_view.animation.claim_frame();
                        }
                    }
                    ToolStyle::OffsetCamera | ToolStyle::Zoom => {}
                }

                widget_flags |= engine_view
                    .document
                    .resize_autoexpand(engine_view.store, engine_view.camera);
                engine_view.store.regenerate_rendering_in_viewport_threaded(
                    engine_view.tasks_tx.clone(),
                    false,
                    engine_view.camera.viewport(),
                    engine_view.camera.image_scale(),
                );

                self.reset(engine_view);

                EventResult {
                    handled: true,
                    propagate: EventPropagation::Stop,
                    progress: PenProgress::Finished,
                }
            }
            (ToolsState::Active, PenEvent::Proximity { .. }) => EventResult {
                handled: false,
                propagate: EventPropagation::Proceed,
                progress: PenProgress::InProgress,
            },
            (ToolsState::Active, PenEvent::KeyPressed { .. }) => EventResult {
                handled: false,
                propagate: EventPropagation::Proceed,
                progress: PenProgress::InProgress,
            },
            (ToolsState::Active, PenEvent::Cancel) => {
                match engine_view.pens_config.tools_config.style {
                    ToolStyle::Laser => {
                        engine_view.animation.claim_frame();
                    }
                    _ => {}
                }

                widget_flags |= engine_view
                    .document
                    .resize_autoexpand(engine_view.store, engine_view.camera);
                engine_view.store.regenerate_rendering_in_viewport_threaded(
                    engine_view.tasks_tx.clone(),
                    false,
                    engine_view.camera.viewport(),
                    engine_view.camera.image_scale(),
                );

                self.reset(engine_view);

                EventResult {
                    handled: true,
                    propagate: EventPropagation::Stop,
                    progress: PenProgress::Finished,
                }
            }
            (ToolsState::Active, PenEvent::Text { .. }) => EventResult {
                handled: false,
                propagate: EventPropagation::Proceed,
                progress: PenProgress::InProgress,
            },
        };

        (event_result, widget_flags)
    }

    fn handle_animation_frame(&mut self, engine_view: &mut EngineViewMut) {
        if !engine_view.store.laser_store.is_faded() {
            engine_view.animation.claim_frame();
        }
    }
}

impl DrawableOnDoc for Tools {
    fn bounds_on_doc(&self, engine_view: &EngineView) -> Option<Aabb> {
        if let ToolStyle::Laser = engine_view.pens_config.tools_config.style {
            self.laser_tool.bounds_on_doc(engine_view)
        } else {
            match self.state {
                ToolsState::Active => match engine_view.pens_config.tools_config.style {
                    ToolStyle::VerticalSpace => self.verticalspace_tool.bounds_on_doc(engine_view),
                    ToolStyle::OffsetCamera => self.offsetcamera_tool.bounds_on_doc(engine_view),
                    ToolStyle::Zoom => self.zoom_tool.bounds_on_doc(engine_view),
                    ToolStyle::Laser => self.laser_tool.bounds_on_doc(engine_view),
                },
                ToolsState::Idle => None,
            }
        }
    }

    fn draw_on_doc(
        &self,
        cx: &mut piet_cairo::CairoRenderContext,
        engine_view: &EngineView,
    ) -> anyhow::Result<()> {
        cx.save().map_err(|e| anyhow::anyhow!("{e:?}"))?;

        match &engine_view.pens_config.tools_config.style {
            ToolStyle::VerticalSpace => {
                self.verticalspace_tool.draw_on_doc(cx, engine_view)?;
            }
            ToolStyle::OffsetCamera => {
                self.offsetcamera_tool.draw_on_doc(cx, engine_view)?;
            }
            ToolStyle::Zoom => {
                self.zoom_tool.draw_on_doc(cx, engine_view)?;
            }
            ToolStyle::Laser => {
                self.laser_tool.draw_on_doc(cx, engine_view)?;
            }
        }

        cx.restore().map_err(|e| anyhow::anyhow!("{e:?}"))?;
        Ok(())
    }
}

impl Tools {
    fn reset(&mut self, engine_view: &mut EngineViewMut) {
        match engine_view.pens_config.tools_config.style {
            ToolStyle::VerticalSpace => {
                self.verticalspace_tool.start_pos_y = 0.0;
                self.verticalspace_tool.pos_y = 0.0;
            }
            ToolStyle::OffsetCamera => {
                self.offsetcamera_tool.start = na::Vector2::zeros();
            }
            ToolStyle::Zoom => {
                self.zoom_tool.start_surface_coord = na::Vector2::zeros();
                self.zoom_tool.current_surface_coord = na::Vector2::zeros();
            }
            ToolStyle::Laser => {
                self.laser_tool.path_builder = None;
            }
        }
        self.state = ToolsState::Idle;
    }
}
