use druid::{
    commands::CLOSE_WINDOW,
    kurbo::BezPath,
    theme,
    widget::{Button, Controller, Flex, Label, Painter, Scroll},
    BoxConstraints, Color, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    Point, RenderContext, Size, UpdateCtx, Widget, WidgetExt, WindowConfig, WindowLevel,
};

struct CloseOnFocusLoss;

impl<T, W: Widget<T>> Controller<T, W> for CloseOnFocusLoss {
    fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        // TODO find replacement for the removed WindowLostFocus event
        // if let Event::WindowLostFocus = event {
        //     ctx.submit_command(CLOSE_WINDOW);
        // }
        child.event(ctx, event, data, env)
    }
}

struct ComboBox<W>(W);

impl<T, W: Widget<T>> Widget<T> for ComboBox<W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.0.event(ctx, event, data, env)
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.0.lifecycle(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        self.0.update(ctx, old_data, data, env)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        self.0.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.0.paint(ctx, data, env);
        let Size { width, height } = ctx.size();
        let l_x = width - height + 7.0;
        let t_y = 10.0;
        let r_x = width - 7.0;
        let m_x = 0.5 * (l_x + r_x);
        let m_y = height - 9.0;
        let mut path = BezPath::new();
        path.move_to((l_x, t_y));
        path.line_to((m_x, m_y));
        path.line_to((r_x, t_y));
        ctx.stroke(path, &env.get(theme::TEXT_COLOR), 2.0);
    }
}

pub fn widget(list: &'static [&'static str]) -> impl Widget<usize> {
    ComboBox(
        Button::new(move |&index: &usize, _: &_| list[index].to_owned())
            .on_click(move |ctx, &mut index: &mut usize, env| {
                ctx.new_sub_window(
                    WindowConfig::default()
                        .show_titlebar(false)
                        .resizable(false)
                        .transparent(true)
                        .window_size(Size::new(
                            ctx.size().width,
                            25.0 * list.len().min(8) as f64 + 2.0,
                        ))
                        .set_position(ctx.to_screen(Point::new(0.0, ctx.size().height - 1.0)))
                        .set_level(WindowLevel::DropDown(ctx.window().clone())),
                    drop_down(list),
                    index,
                    env.clone(),
                );
            })
            .env_scope(|env, _| {
                env.set(theme::BUTTON_BORDER_RADIUS, 0.0);
                env.set(theme::BUTTON_LIGHT, Color::grey8(0x10));
                env.set(theme::BUTTON_DARK, Color::grey8(0x10));
            }),
    )
}

fn drop_down(list: &'static [&'static str]) -> impl Widget<usize> {
    let mut flex = Flex::column();
    for (index, &item) in list.iter().enumerate() {
        let label = Label::new(item)
            .expand_width()
            .center()
            .fix_height(25.0)
            .padding((5.0, 0.0))
            .background(Painter::new(move |ctx, selected_index, env| {
                let shape = ctx.size().to_rect();
                if ctx.is_hot() {
                    ctx.fill(shape, &Color::rgb8(30, 144, 255));
                }
            }))
            .on_click(move |ctx, selected_index, env| {
                *selected_index = index;
                ctx.submit_command(CLOSE_WINDOW);
            });
        flex.add_child(label);
    }
    Scroll::new(flex)
        .vertical()
        .background(Color::grey8(0x10))
        .border(Color::grey8(0x50), 1.0)
        .controller(CloseOnFocusLoss)
}
