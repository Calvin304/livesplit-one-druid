use std::{cell::RefCell, rc::Rc};

use druid::{
    kurbo::PathEl,
    piet::{Device, ImageFormat, InterpolationMode, PaintBrush, PietImage},
    Affine, Color, ImageBuf, LinearGradient, PaintCtx, Point, Rect, RenderContext, UnitPoint,
};
use livesplit_core::{layout::LayoutState, rendering::software::Renderer};

pub fn render_scene(
    paint_ctx: &mut PaintCtx,
    renderer: &mut Renderer,
    state: &LayoutState,
) -> Option<(f32, f32)> {
    let size = paint_ctx.size();
    let (width, height) = (size.width as u32, size.height as u32);
    // let dimensions = renderer.image().dimensions();

    let new_dims = renderer.render(state, [width, height]);

    // PietImage doesnt currently have an api to reuse previous buffers (or i couldnt find one)
    // so just unconditionally make a new one for now
    // TODO find a way to solve the above

    let image = paint_ctx
        .make_image(
            width as usize,
            height as usize,
            renderer.image_data(),
            ImageFormat::RgbaPremul,
        )
        .ok()?;

    paint_ctx.draw_image(
        &image,
        Rect::from_origin_size(Point::ZERO, size),
        InterpolationMode::NearestNeighbor,
    );

    new_dims
}
