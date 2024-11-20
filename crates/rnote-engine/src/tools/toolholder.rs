use gtk4::prelude::SnapshotExt;
use p2d::bounding_volume::Aabb;
use piet::{RenderContext, TextLayoutBuilder};
use piet::{Text, TextLayout};
use rnote_compose::ext::{AabbExt, Vector2Ext};

use crate::ext::GrapheneRectExt;
use crate::render::Image;
use crate::{
    document::format::MeasureUnit, drawable::DrawableOnSurface, engine::EngineView, Camera,
    WidgetFlags,
};

pub trait Draggable {
    fn is_point_in_drag_area(&self, point: kurbo::Point, camera: &Camera) -> bool;
    fn offset(&self) -> na::Vector2<f64>;
    fn drag(&mut self, offset: na::Vector2<f64>) -> WidgetFlags;
}

pub trait Rotatable {
    fn angle(&self) -> f64;
    fn set_angle(&mut self, angle: f64) -> WidgetFlags;
}

pub trait Constraining {
    fn constrain(&self, point: na::Point2<f64>, camera: &Camera) -> na::Point2<f64>;
}

#[derive(Debug, Default)]
pub struct ToolHolder {
    pub current_tool: Tool,
}

impl Draggable for ToolHolder {
    fn is_point_in_drag_area(&self, point: kurbo::Point, camera: &Camera) -> bool {
        self.current_tool.is_point_in_drag_area(point, camera)
    }

    fn offset(&self) -> na::Vector2<f64> {
        self.current_tool.offset()
    }

    fn drag(&mut self, offset: na::Vector2<f64>) -> WidgetFlags {
        match &mut self.current_tool {
            Tool::Ruler(ruler) => ruler.drag(offset),
        }
    }
}

impl Rotatable for ToolHolder {
    fn angle(&self) -> f64 {
        self.current_tool.angle()
    }

    fn set_angle(&mut self, angle: f64) -> WidgetFlags {
        match &mut self.current_tool {
            Tool::Ruler(ruler) => ruler.set_angle(angle),
        }
    }
}

impl Constraining for ToolHolder {
    fn constrain(&self, point: na::Point2<f64>, camera: &Camera) -> na::Point2<f64> {
        match &self.current_tool {
            Tool::Ruler(ruler) => ruler.constrain(point, camera),
        }
    }
}

impl DrawableOnSurface for ToolHolder {
    fn bounds_on_surface(&self, engine_view: &EngineView) -> Option<Aabb> {
        self.current_tool.bounds_on_surface(engine_view)
    }

    fn gen_image(&self, scale_factor: f64, engine_view: &EngineView) -> anyhow::Result<Image> {
        self.current_tool.gen_image(scale_factor, engine_view)
    }

    fn draw_on_surface_to_gtk_snapshot(
        &self,
        snapshot: &gtk4::Snapshot,
        base_rendernode: &gtk4::gsk::RenderNode,
        engine_view: &EngineView,
    ) -> anyhow::Result<()> {
        snapshot.save();

        self.current_tool.draw_on_surface_to_gtk_snapshot(
            snapshot,
            base_rendernode,
            engine_view,
        )?;

        snapshot.restore();
        Ok(())
    }
}

#[derive(Debug)]
pub enum Tool {
    Ruler(Ruler),
}

impl Draggable for Tool {
    fn is_point_in_drag_area(&self, point: kurbo::Point, camera: &Camera) -> bool {
        match self {
            Tool::Ruler(ruler) => ruler.is_point_in_drag_area(point, camera),
        }
    }

    fn offset(&self) -> na::Vector2<f64> {
        match self {
            Tool::Ruler(ruler) => ruler.offset(),
        }
    }

    fn drag(&mut self, offset: na::Vector2<f64>) -> WidgetFlags {
        match self {
            Tool::Ruler(ruler) => ruler.drag(offset),
        }
    }
}

impl Rotatable for Tool {
    fn angle(&self) -> f64 {
        match self {
            Tool::Ruler(ruler) => ruler.angle(),
        }
    }

    fn set_angle(&mut self, angle: f64) -> WidgetFlags {
        match self {
            Tool::Ruler(ruler) => ruler.set_angle(angle),
        }
    }
}

impl DrawableOnSurface for Tool {
    fn bounds_on_surface(&self, engine_view: &EngineView) -> Option<p2d::bounding_volume::Aabb> {
        match self {
            Tool::Ruler(ruler) => ruler.bounds_on_surface(engine_view),
        }
    }

    fn gen_image(&self, scale_factor: f64, engine_view: &EngineView) -> anyhow::Result<Image> {
        match self {
            Tool::Ruler(ruler) => ruler.gen_image(scale_factor, engine_view),
        }
    }

    fn draw_on_surface_to_gtk_snapshot(
        &self,
        snapshot: &gtk4::Snapshot,
        base_rendernode: &gtk4::gsk::RenderNode,

        engine_view: &EngineView,
    ) -> anyhow::Result<()> {
        match self {
            Tool::Ruler(ruler) => {
                ruler.draw_on_surface_to_gtk_snapshot(snapshot, base_rendernode, engine_view)
            }
        }
    }
}

impl Default for Tool {
    fn default() -> Self {
        Self::Ruler(Ruler::default())
    }
}

#[derive(Debug)]
pub struct Ruler {
    pub offset: na::Vector2<f64>,
    pub angle: f64,
}

impl Ruler {
    const WIDTH: f64 = 100.0;
    const LINE_WIDTH: f64 = 1.5;

    fn format_angle(&self) -> String {
        let mut angle = self.angle.to_degrees().round() as u16 % 180;

        if angle > 90 {
            angle = 180 - angle;
        }

        return angle.to_string();
    }

    pub fn bounds(camera: &Camera) -> Aabb {
        let camera_size = camera.size();
        let diagonal_length = camera_size.norm();

        Aabb::new(
            na::point![-diagonal_length, -Self::WIDTH / 2.0],
            na::point![diagonal_length, Self::WIDTH / 2.0],
        )
    }
}

impl Default for Ruler {
    fn default() -> Self {
        Self {
            offset: na::Vector2::new(0.5, 0.5),
            angle: 0.0,
        }
    }
}

impl Draggable for Ruler {
    fn is_point_in_drag_area(&self, point: kurbo::Point, camera: &Camera) -> bool {
        let camera_size = camera.size();

        let diagonal_length = camera_size.norm();

        let rect = kurbo::Rect::new(
            -diagonal_length,
            -Self::WIDTH / 2.0,
            diagonal_length,
            Self::WIDTH / 2.0,
        );

        let transform = kurbo::Affine::rotate(self.angle)
            .then_translate((camera_size.component_mul(&self.offset)).to_kurbo_vec());

        let point = transform.inverse() * point;

        rect.contains(point)
    }

    fn offset(&self) -> na::Vector2<f64> {
        self.offset
    }

    fn drag(&mut self, mut offset: na::Vector2<f64>) -> WidgetFlags {
        let mut widget_flags = WidgetFlags::default();

        offset.x = offset.x.clamp(0.0, 1.0);
        offset.y = offset.y.clamp(0.0, 1.0);

        self.offset = offset;

        widget_flags.redraw = true;
        widget_flags
    }
}

impl Rotatable for Ruler {
    fn angle(&self) -> f64 {
        self.angle
    }

    fn set_angle(&mut self, angle: f64) -> WidgetFlags {
        let mut widget_flags = WidgetFlags::default();

        self.angle = (angle.to_degrees().round() % 180.0).to_radians();

        widget_flags.redraw = true;
        widget_flags
    }
}

impl Constraining for Ruler {
    fn constrain(&self, point: na::Point2<f64>, camera: &Camera) -> na::Point2<f64> {
        let camera_size = camera.size();

        let transform = kurbo::Affine::rotate(self.angle)
            .then_translate((camera_size.component_mul(&self.offset)).to_kurbo_vec());

        let point_local =
            transform.inverse() * (camera.transform() * point).coords.to_kurbo_point();

        let mut point_constrained = point_local;
        if point_constrained.y.abs() < Self::WIDTH / 2.0 {
            point_constrained.y = point_constrained.y.signum() * Self::WIDTH / 2.0;
        }

        let point_world = transform * point_constrained;

        camera.transform().inverse() * na::point![point_world.x, point_world.y]
    }
}

impl DrawableOnSurface for Ruler {
    fn bounds_on_surface(&self, engine_view: &EngineView) -> Option<Aabb> {
        let camera_size = engine_view.camera.size();

        let angle_text_rect = kurbo::Rect::new(
            -Self::WIDTH / 2.0,
            -Self::WIDTH / 2.0,
            Self::WIDTH / 2.0,
            Self::WIDTH / 2.0,
        );

        let transform = kurbo::Affine::rotate(self.angle)
            .then_translate((camera_size.component_mul(&self.offset)).to_kurbo_vec());

        Some(Aabb::from_kurbo_rect(
            transform.transform_rect_bbox(angle_text_rect),
        ))
    }

    fn gen_image(&self, scale_factor: f64, engine_view: &EngineView) -> anyhow::Result<Image> {
        const MM_LENGTH: f64 = 10.0;

        let bounds = Self::bounds(engine_view.camera);

        Image::gen_with_piet(
            |cx| {
                let zoom = engine_view.camera.total_zoom();
                let camera_size = engine_view.camera.size();

                let diagonal_length = camera_size.norm();

                let rect = bounds.to_kurbo_rect();
                cx.fill(rect, &piet::Color::GRAY.with_a8(128));

                let mm_in_pixel = MeasureUnit::convert_measurement(
                    1.0,
                    MeasureUnit::Mm,
                    engine_view.document.format.dpi(),
                    MeasureUnit::Px,
                    engine_view.document.format.dpi(),
                ) * zoom;

                let mm_steps = 0..(diagonal_length / mm_in_pixel) as usize;

                let step_size = if zoom < 1.0 { 5 } else { 1 };

                for i in mm_steps.step_by(step_size) {
                    let x = i as f64 * mm_in_pixel;

                    let length = match i % 10 {
                        0 => MM_LENGTH * 3.0,
                        5 => MM_LENGTH * 2.0,
                        _ => MM_LENGTH,
                    };

                    let mark1 =
                        kurbo::Line::new((x, Self::WIDTH / 2.0 - length), (x, Self::WIDTH / 2.0));
                    let mark2 =
                        kurbo::Line::new((-x, Self::WIDTH / 2.0 - length), (-x, Self::WIDTH / 2.0));

                    let mark3 =
                        kurbo::Line::new((x, length - Self::WIDTH / 2.0), (x, -Self::WIDTH / 2.0));
                    let mark4 = kurbo::Line::new(
                        (-x, length - Self::WIDTH / 2.0),
                        (-x, -Self::WIDTH / 2.0),
                    );

                    cx.stroke(mark1, &piet::Color::BLACK, Self::LINE_WIDTH);
                    cx.stroke(mark2, &piet::Color::BLACK, Self::LINE_WIDTH);
                    cx.stroke(mark3, &piet::Color::BLACK, Self::LINE_WIDTH);
                    cx.stroke(mark4, &piet::Color::BLACK, Self::LINE_WIDTH);
                }

                Ok(())
            },
            bounds,
            scale_factor,
        )
    }

    fn draw_on_surface_to_gtk_snapshot(
        &self,
        snapshot: &gtk4::Snapshot,
        base_rendernode: &gtk4::gsk::RenderNode,
        engine_view: &EngineView,
    ) -> anyhow::Result<()> {
        let camera_size = engine_view.camera.size();
        let position = camera_size.component_mul(&self.offset);

        snapshot.save();

        snapshot.transform(Some(
            &gtk4::gsk::Transform::new()
                .translate(&gtk4::graphene::Point::new(
                    position.x as f32,
                    position.y as f32,
                ))
                .rotate(self.angle.to_degrees() as f32)
                .scale(engine_view.camera.temporary_zoom() as f32, 1.0),
        ));

        snapshot.append_node(base_rendernode);

        let num_extensions = engine_view.camera.temporary_zoom().recip().ceil() as u32;

        if num_extensions > 1 {
            let width_rendernode = base_rendernode.bounds().width() - Self::LINE_WIDTH as f32;

            snapshot.save();
            for _ in 1..num_extensions {
                snapshot.translate(&gtk4::graphene::Point::new(width_rendernode, 0.0));
                snapshot.append_node(base_rendernode);
            }
            snapshot.restore();

            snapshot.save();
            for _ in 1..num_extensions {
                snapshot.translate(&gtk4::graphene::Point::new(-width_rendernode, 0.0));
                snapshot.append_node(base_rendernode);
            }
            snapshot.restore();
        }

        snapshot.restore();

        if let Some(angle_text_bounds) = self.bounds_on_surface(engine_view) {
            let cairo_cx =
                snapshot.append_cairo(&gtk4::graphene::Rect::from_p2d_aabb(angle_text_bounds));
            let mut piet_cx = piet_cairo::CairoRenderContext::new(&cairo_cx);

            let origin_indicator = kurbo::Circle::new(position.to_kurbo_point(), 20.0);
            piet_cx.stroke(origin_indicator, &piet::Color::BLACK, Self::LINE_WIDTH);

            let angle_text_layout = piet_cx
                .text()
                .new_text_layout(self.format_angle())
                .font(piet::FontFamily::SYSTEM_UI, 16.0)
                .alignment(piet::TextAlignment::Center)
                .text_color(piet::Color::BLACK)
                .build()
                .unwrap();

            let angle_text_size = angle_text_layout.size();

            piet_cx.draw_text(
                &angle_text_layout,
                (
                    position.x - angle_text_size.width / 2.0,
                    position.y - angle_text_size.height / 2.0,
                ),
            );
        }

        Ok(())
    }
}
