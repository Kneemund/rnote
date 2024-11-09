use kurbo::Shape;
use p2d::bounding_volume::Aabb;
use piet::RenderContext;
use piet_cairo::CairoRenderContext;
use rnote_compose::ext::{AabbExt, Vector2Ext};

use crate::{
    document::format::MeasureUnit, drawable::DrawableOnSurface, engine::EngineView, Camera,
    WidgetFlags,
};

pub trait Draggable {
    fn is_point_in_drag_area(&self, point: na::Point2<f64>, camera: &Camera) -> bool;
    fn offset(&self) -> na::Vector2<f64>;
    fn drag(&mut self, offset: na::Vector2<f64>) -> WidgetFlags;
}

#[derive(Debug, Default)]
pub struct ToolHolder {
    pub current_tool: Tool,
}

impl Draggable for ToolHolder {
    fn is_point_in_drag_area(&self, point: na::Point2<f64>, camera: &Camera) -> bool {
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

impl DrawableOnSurface for ToolHolder {
    fn bounds_on_surface(&self, engine_view: &EngineView) -> Option<Aabb> {
        self.current_tool.bounds_on_surface(engine_view)
    }
    fn draw_on_surface(
        &self,
        cx: &mut piet_cairo::CairoRenderContext,
        engine_view: &EngineView,
    ) -> anyhow::Result<()> {
        cx.save().map_err(|e| anyhow::anyhow!("{e:?}"))?;

        self.current_tool.draw_on_surface(cx, engine_view)?;

        cx.restore().map_err(|e| anyhow::anyhow!("{e:?}"))?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum Tool {
    Ruler(Ruler),
}

impl Draggable for Tool {
    fn is_point_in_drag_area(&self, point: na::Point2<f64>, camera: &Camera) -> bool {
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

impl DrawableOnSurface for Tool {
    fn bounds_on_surface(&self, engine_view: &EngineView) -> Option<p2d::bounding_volume::Aabb> {
        match self {
            Tool::Ruler(ruler) => ruler.bounds_on_surface(engine_view),
        }
    }

    fn draw_on_surface(
        &self,
        cx: &mut CairoRenderContext,
        engine_view: &EngineView,
    ) -> anyhow::Result<()> {
        match self {
            Tool::Ruler(ruler) => ruler.draw_on_surface(cx, engine_view),
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
}

impl Default for Ruler {
    fn default() -> Self {
        Self {
            offset: na::Vector2::new(0.5, 0.5),
            angle: std::f64::consts::PI / 4.0,
        }
    }
}

impl Draggable for Ruler {
    fn is_point_in_drag_area(&self, point: na::Point2<f64>, camera: &Camera) -> bool {
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

        let point = transform.inverse() * kurbo::Point::new(point.x, point.y);

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

impl DrawableOnSurface for Ruler {
    fn bounds_on_surface(&self, engine_view: &EngineView) -> Option<Aabb> {
        let camera_size = engine_view.camera.size();

        let diagonal_length = camera_size.norm();

        let rect = kurbo::Rect::new(
            -diagonal_length,
            -Self::WIDTH / 2.0,
            diagonal_length,
            Self::WIDTH / 2.0,
        );

        let transform = kurbo::Affine::rotate(self.angle)
            .then_translate((camera_size.component_mul(&self.offset)).to_kurbo_vec());

        Some(Aabb::from_kurbo_rect(transform.transform_rect_bbox(rect)))
    }

    fn draw_on_surface(
        &self,
        cx: &mut piet_cairo::CairoRenderContext,
        engine_view: &EngineView,
    ) -> anyhow::Result<()> {
        const MM_LENGTH: f64 = 10.0;
        const MM_WIDTH: f64 = 1.0;

        let zoom = engine_view.camera.total_zoom();
        let camera_size = engine_view.camera.size();

        let diagonal_length = camera_size.norm();

        cx.transform(
            kurbo::Affine::rotate(self.angle)
                .then_translate((camera_size.component_mul(&self.offset)).to_kurbo_vec()),
        );

        let rect = kurbo::Rect::new(
            -diagonal_length,
            -Self::WIDTH / 2.0,
            diagonal_length,
            Self::WIDTH / 2.0,
        )
        .to_path(0.5);

        cx.fill(rect, &piet::Color::GRAY.with_a8(128));

        let origin_indicator = kurbo::Circle::new((0.0, 0.0), 5.0);
        cx.stroke(origin_indicator, &piet::Color::BLACK, MM_WIDTH);

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

            let mark1 = kurbo::Line::new((x, Self::WIDTH / 2.0 - length), (x, Self::WIDTH / 2.0));
            let mark2 = kurbo::Line::new((-x, Self::WIDTH / 2.0 - length), (-x, Self::WIDTH / 2.0));

            let mark3 = kurbo::Line::new((x, length - Self::WIDTH / 2.0), (x, -Self::WIDTH / 2.0));
            let mark4 =
                kurbo::Line::new((-x, length - Self::WIDTH / 2.0), (-x, -Self::WIDTH / 2.0));

            cx.stroke(mark1, &piet::Color::BLACK, MM_WIDTH);
            cx.stroke(mark2, &piet::Color::BLACK, MM_WIDTH);
            cx.stroke(mark3, &piet::Color::BLACK, MM_WIDTH);
            cx.stroke(mark4, &piet::Color::BLACK, MM_WIDTH);
        }

        Ok(())
    }
}
