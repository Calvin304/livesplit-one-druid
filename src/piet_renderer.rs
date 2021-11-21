use std::{cell::RefCell, rc::Rc};

use druid::{
    kurbo::PathEl,
    piet::{Device, ImageFormat, InterpolationMode, PaintBrush, PietImage},
    Affine, Color, ImageBuf, LinearGradient, PaintCtx, Rect, RenderContext, UnitPoint,
};
use livesplit_core::rendering::{
    Entity, FillShader, PathBuilder, ResourceAllocator, Rgba, Scene, Transform,
};

pub struct PietResourceAllocator;

pub struct PietPathBuilder(Vec<PathEl>);

impl PathBuilder<PietResourceAllocator> for PietPathBuilder {
    type Path = Rc<[PathEl]>;

    fn move_to(&mut self, x: f32, y: f32) {
        self.0.push(PathEl::MoveTo((x as f64, y as f64).into()))
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.0.push(PathEl::LineTo((x as f64, y as f64).into()))
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.0.push(PathEl::QuadTo(
            (x1 as f64, y1 as f64).into(),
            (x as f64, y as f64).into(),
        ))
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.0.push(PathEl::CurveTo(
            (x1 as f64, y1 as f64).into(),
            (x2 as f64, y2 as f64).into(),
            (x as f64, y as f64).into(),
        ))
    }

    fn close(&mut self) {
        self.0.push(PathEl::ClosePath)
    }

    fn finish(self, _alloc: &mut PietResourceAllocator) -> Self::Path {
        self.0.into()
    }
}

fn convert_color(&[r, g, b, a]: &Rgba) -> Color {
    Color::rgba(r as _, g as _, b as _, a as _)
}

fn convert_transform(transform: &Transform) -> Affine {
    let [sx, ky, kx, sy, tx, ty] = transform.to_array();
    Affine::new([sx as _, ky as _, kx as _, sy as _, tx as _, ty as _])
}

fn convert_shader(shader: &FillShader) -> PaintBrush {
    match shader {
        FillShader::SolidColor(c) => PaintBrush::Color(convert_color(c)),
        FillShader::VerticalGradient(t, b) => PaintBrush::Linear(LinearGradient::new(
            UnitPoint::TOP,
            UnitPoint::BOTTOM,
            (convert_color(t), convert_color(b)),
        )),
        FillShader::HorizontalGradient(l, r) => PaintBrush::Linear(LinearGradient::new(
            UnitPoint::LEFT,
            UnitPoint::RIGHT,
            (convert_color(l), convert_color(r)),
        )),
    }
}

pub struct Image {
    buf: ImageBuf,
    layers: RefCell<[Option<PietImage>; 2]>,
}

impl ResourceAllocator for PietResourceAllocator {
    type PathBuilder = PietPathBuilder;
    type Path = Rc<[PathEl]>;
    type Image = Rc<Image>;

    fn path_builder(&mut self) -> Self::PathBuilder {
        PietPathBuilder(Vec::new())
    }

    fn create_image(&mut self, width: u32, height: u32, data: &[u8]) -> Self::Image {
        Rc::new(Image {
            buf: ImageBuf::from_raw(
                data,
                ImageFormat::RgbaSeparate,
                width as usize,
                height as usize,
            ),
            layers: RefCell::new([None, None]),
        })
    }
}

// pub fn render_scene(ctx: &mut PaintCtx, scene: &Scene<Rc<[PathEl]>, ()>) {
//     if let Some(background) = scene.background() {
//         // TODO: We could use .clear(...) maybe
//         let rect = ctx.size().to_rect();
//         ctx.fill(rect, &convert_shader(background));
//     }

//     scene
//         .bottom_layer()
//         .iter()
//         .chain(scene.top_layer())
//         .for_each(|element| match element {
//             Entity::FillPath(path, shader, transform) => {
//                 ctx.with_save(|ctx| {
//                     ctx.transform(convert_transform(transform));
//                     ctx.fill(&***path, &convert_shader(shader));
//                 });
//             }
//             Entity::StrokePath(path, stroke_width, color, transform) => {
//                 ctx.with_save(|ctx| {
//                     ctx.transform(convert_transform(transform));
//                     ctx.stroke(
//                         &***path,
//                         &PaintBrush::Color(convert_color(color)),
//                         *stroke_width as f64,
//                     );
//                 });
//             }
//             Entity::Image(image, transform) => {}
//         });
// }

fn render_layer(
    ctx: &mut impl RenderContext<Image = PietImage>,
    layer: &[Entity<Rc<[PathEl]>, Rc<Image>>],
    is_top_layer: bool,
) {
    for entity in layer {
        match entity {
            Entity::FillPath(path, shader, transform) => {
                let _ = ctx.with_save(|ctx| {
                    ctx.transform(convert_transform(transform));
                    ctx.fill(&***path, &convert_shader(shader));
                    Ok(())
                });
            }
            Entity::StrokePath(path, stroke_width, color, transform) => {
                let _ = ctx.with_save(|ctx| {
                    ctx.transform(convert_transform(transform));
                    ctx.stroke(
                        &***path,
                        &PaintBrush::Color(convert_color(color)),
                        *stroke_width as f64,
                    );
                    Ok(())
                });
            }
            Entity::Image(image, transform) => {
                let mut layers = image.layers.borrow_mut();
                let image =
                    layers[is_top_layer as usize].get_or_insert_with(|| image.buf.to_image(ctx));
                let _ = ctx.with_save(|ctx| {
                    ctx.transform(convert_transform(transform));
                    ctx.draw_image(
                        image,
                        Rect::new(0.0, 0.0, 1.0, 1.0),
                        InterpolationMode::Bilinear,
                    );
                    Ok(())
                });
            }
        }
    }
}

pub fn render_scene(
    paint_ctx: &mut PaintCtx,
    bottom_image: &mut PietImage,
    device: &mut Device,
    scene: &Scene<Rc<[PathEl]>, Rc<Image>>,
) {
    let size = paint_ctx.size();
    let (width, height) = (size.width, size.height);

    // TODO: Bring this back. Potentially requires duplicating images across top
    // and bottom layer.

    if scene.bottom_layer_changed() {
        let mut target = device
            .bitmap_target(width.ceil() as _, height.ceil() as _, 1.0)
            .unwrap();
        {
            let mut ctx = target.render_context();

            // TODO: We shouldn't clear and then fill
            ctx.clear(None, Color::TRANSPARENT);
            if let Some(background) = scene.background() {
                let rect = Rect::new(0.0, 0.0, width, height);
                ctx.fill(rect, &convert_shader(background));
            }

            render_layer(&mut ctx, scene.bottom_layer(), false);

            ctx.finish();
        }
        *bottom_image = target
            .to_image_buf(ImageFormat::RgbaPremul)
            .unwrap()
            .to_image(paint_ctx.render_ctx);
    }

    paint_ctx.draw_image(
        bottom_image,
        Rect::new(0.0, 0.0, width, height),
        InterpolationMode::NearestNeighbor,
    );

    render_layer(paint_ctx.render_ctx, scene.top_layer(), true);
}
