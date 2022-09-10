use std::{
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
};

use druid::{
    commands,
    piet::{Device, ImageFormat, PietImage},
    theme,
    widget::{Controller, Flex},
    AppDelegate, AppLauncher, BoxConstraints, DelegateCtx, Env, Event, EventCtx, FileDialogOptions,
    FileInfo, FileSpec, LayoutCtx, LifeCycle, LifeCycleCtx, LocalizedString, Menu, MenuItem,
    MouseButton, Point, RenderContext, Selector, Size, UpdateCtx, Widget, WidgetExt, WindowDesc,
    WindowId, WindowLevel,
};
use livesplit_core::{
    layout::{self, LayoutSettings},
    run::parser::composite,
    Layout, LayoutEditor, RunEditor,
};

use crate::{
    consts::{
        BACKGROUND, BUTTON_BORDER, BUTTON_BORDER_RADIUS, BUTTON_BOTTOM, BUTTON_TOP, MARGIN,
        PRIMARY_LIGHT, SELECTED_TEXT_BACKGROUND_COLOR, TEXTBOX_BACKGROUND,
    },
    layout_editor, run_editor, settings_editor, software_renderer, LayoutEditorLens, MainState,
    OpenWindow, RunEditorLens, SettingsEditorLens,
};

struct WithMenu<T> {
    // device: Device,
    renderer: livesplit_core::rendering::software::Renderer,
    inner: T,
}

impl<T> WithMenu<T> {
    fn new(inner: T) -> Self {
        // let mut device = Device::new().unwrap();
        Self {
            // bottom_image: {
            //     let mut target = device.bitmap_target(1, 1, 1.0).unwrap();
            //     let mut ctx = target.render_context();
            //     let image = ctx
            //         .make_image(1, 1, &[0; 4], ImageFormat::RgbaPremul)
            //         .unwrap();
            //     ctx.finish().unwrap();
            //     image
            // },
            // device,
            renderer: Default::default(),
            inner,
        }
    }
}

const CONTEXT_MENU_EDIT_SPLITS: Selector = Selector::new("context-menu-edit-splits");
const CONTEXT_MENU_OPEN_SPLITS: Selector<FileInfo> = Selector::new("context-menu-open-splits");
const CONTEXT_MENU_EDIT_LAYOUT: Selector = Selector::new("context-menu-edit-layout");
const CONTEXT_MENU_OPEN_LAYOUT: Selector<FileInfo> = Selector::new("context-menu-open-layout");
const CONTEXT_MENU_START_OR_SPLIT: Selector = Selector::new("context-menu-start-or-split");
const CONTEXT_MENU_RESET: Selector = Selector::new("context-menu-reset");
const CONTEXT_MENU_UNDO_SPLIT: Selector = Selector::new("context-menu-undo-split");
const CONTEXT_MENU_SKIP_SPLIT: Selector = Selector::new("context-menu-skip-split");
const CONTEXT_MENU_TOGGLE_PAUSE: Selector = Selector::new("context-menu-toggle-pause");
const CONTEXT_MENU_UNDO_ALL_PAUSES: Selector = Selector::new("context-menu-undo-all-pauses");
const CONTEXT_MENU_TOGGLE_TIMING_METHOD: Selector =
    Selector::new("context-menu-toggle-timing-method");
const CONTEXT_MENU_SET_COMPARISON: Selector<String> = Selector::new("context-menu-set-comparison");
const CONTEXT_MENU_EDIT_SETTINGS: Selector = Selector::new("context-menu-edit-settings");

impl<T: Widget<MainState>> Widget<MainState> for WithMenu<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut MainState, env: &Env) {
        match event {
            Event::AnimFrame(_) => {
                ctx.request_anim_frame();
                ctx.request_paint();
            }
            Event::Wheel(event) => {
                if event.wheel_delta.y > 0.0 {
                    data.layout_data.borrow_mut().layout.scroll_down();
                } else {
                    data.layout_data.borrow_mut().layout.scroll_up();
                }
            }
            Event::MouseUp(event) => {
                if event.button == MouseButton::Right
                    && data.run_editor.is_none()
                    && data.layout_editor.is_none()
                    && data.settings_editor.is_none()
                {
                    let mut compare_against = Menu::new("Compare Against");

                    //TODO dont unwrap
                    let timer = data.timer.read().unwrap();
                    let current_comparison = timer.current_comparison();
                    for comparison in timer.run().comparisons() {
                        compare_against = compare_against.entry(
                            MenuItem::new(comparison)
                                .command(CONTEXT_MENU_SET_COMPARISON.with(comparison.to_owned()))
                                .selected(comparison == current_comparison),
                        );
                    }

                    ctx.show_context_menu::<MainState>(
                        Menu::new("LiveSplit")
                            .entry(
                                MenuItem::new("Edit Splits...").command(CONTEXT_MENU_EDIT_SPLITS),
                            )
                            .entry(
                                MenuItem::new("Open Splits...").command(
                                    commands::SHOW_OPEN_PANEL.with(
                                        FileDialogOptions::new()
                                            .title("Open Splits")
                                            .accept_command(CONTEXT_MENU_OPEN_SPLITS),
                                    ),
                                ),
                            )
                            .entry(MenuItem::new("Save Splits").command(
                                commands::SHOW_SAVE_PANEL.with(
                                    FileDialogOptions::new().title("Save Splits").allowed_types(
                                        vec![
                                            FileSpec {
                                                name: "LiveSplit Splits",
                                                extensions: &["lss"],
                                            },
                                            FileSpec {
                                                name: "All Files",
                                                extensions: &["*.*"],
                                            },
                                        ],
                                    ),
                                ),
                            ))
                            .entry(
                                MenuItem::new("Save Splits As...")
                                    .command(CONTEXT_MENU_EDIT_SPLITS),
                            )
                            .separator()
                            .entry(
                                Menu::new("Control")
                                    .entry(
                                        MenuItem::new("Start / Split")
                                            .command(CONTEXT_MENU_START_OR_SPLIT),
                                    )
                                    .entry(MenuItem::new("Reset").command(CONTEXT_MENU_RESET))
                                    .entry(
                                        MenuItem::new("Undo Split")
                                            .command(CONTEXT_MENU_UNDO_SPLIT),
                                    )
                                    .entry(
                                        MenuItem::new("Skip Split")
                                            .command(CONTEXT_MENU_SKIP_SPLIT),
                                    )
                                    .entry(
                                        MenuItem::new("Toggle Pause")
                                            .command(CONTEXT_MENU_TOGGLE_PAUSE),
                                    )
                                    .entry(
                                        MenuItem::new("Undo All Pauses")
                                            .command(CONTEXT_MENU_UNDO_ALL_PAUSES),
                                    )
                                    .entry(
                                        MenuItem::new("Toggle Timing Method")
                                            .command(CONTEXT_MENU_TOGGLE_TIMING_METHOD),
                                    ),
                            )
                            .entry(compare_against)
                            .separator()
                            .entry(
                                MenuItem::new("Edit Layout...").command(CONTEXT_MENU_EDIT_LAYOUT),
                            )
                            .entry(
                                MenuItem::new("Open Layout...").command(
                                    commands::SHOW_OPEN_PANEL.with(
                                        FileDialogOptions::new()
                                            .title("Open Layout")
                                            .allowed_types(vec![
                                                FileSpec {
                                                    name: "LiveSplit Layouts",
                                                    extensions: &["lsl", "ls1l"],
                                                },
                                                FileSpec {
                                                    name: "All Files",
                                                    extensions: &["*.*"],
                                                },
                                            ])
                                            .accept_command(CONTEXT_MENU_OPEN_LAYOUT),
                                    ),
                                ),
                            )
                            .entry(MenuItem::new("Save Layout").command(CONTEXT_MENU_EDIT_SPLITS))
                            .entry(
                                MenuItem::new("Save Layout As...")
                                    .command(CONTEXT_MENU_EDIT_SPLITS),
                            )
                            .separator()
                            .entry(MenuItem::new("Settings").command(CONTEXT_MENU_EDIT_SETTINGS))
                            .separator()
                            // .entry(MenuItem::new("About").command(CONTEXT_MENU_EDIT_SPLITS))
                            .entry(MenuItem::new("Exit").command(commands::QUIT_APP)),
                        event.pos,
                    );
                }
            }
            Event::Command(command) => {
                if command.is(CONTEXT_MENU_EDIT_SPLITS) {
                    // the only error is threadstopped which means the hotkey system is effectively disabled anyways
                    let _ = data.hotkey_system.borrow_mut().deactivate();
                    // TODO dont unwrap
                    let run = data.timer.read().unwrap().run().clone();
                    let editor = RunEditor::new(run).unwrap();
                    let window = WindowDesc::new(run_editor::root_widget().lens(RunEditorLens))
                        .title("Splits Editor")
                        .with_min_size((690.0, 495.0))
                        .window_size((690.0, 495.0))
                        .set_level(WindowLevel::Modal(ctx.window().clone()));
                    let window_id = window.id;
                    ctx.new_window(window);
                    data.run_editor = Some(OpenWindow {
                        id: window_id,
                        state: run_editor::State::new(editor),
                    });
                } else if let Some(file_info) = command.get(CONTEXT_MENU_OPEN_SPLITS) {
                    let mut file = File::open(file_info.path()).unwrap();
                    let mut file_contents = Vec::new();
                    let _size = file.read_to_end(&mut file_contents);
                    let run = composite::parse(
                        file_contents.as_slice(),
                        Some(file_info.path().to_path_buf()),
                        true,
                    )
                    .unwrap();
                    //TODO dont unwrap
                    data.timer
                        .write()
                        .unwrap()
                        .set_run(run.run)
                        .map_err(drop)
                        .unwrap();
                    data.config
                        .borrow_mut()
                        .set_splits_path(Some(file_info.path()));
                } else if command.is(CONTEXT_MENU_EDIT_LAYOUT) {
                    data.hotkey_system.borrow_mut().deactivate();
                    let layout = data.layout_data.borrow().layout.clone();
                    let editor = LayoutEditor::new(layout).unwrap();
                    let window =
                        WindowDesc::new(layout_editor::root_widget().lens(LayoutEditorLens))
                            .title("Layout Editor")
                            .with_min_size((500.0, 600.0))
                            .window_size((550.0, 650.0))
                            .set_level(WindowLevel::Modal(ctx.window().clone()));
                    let window_id = window.id;
                    ctx.new_window(window);
                    data.layout_editor = Some(OpenWindow {
                        id: window_id,
                        state: layout_editor::State::new(editor),
                    });
                } else if let Some(file_info) = command.get(CONTEXT_MENU_OPEN_LAYOUT) {
                    let mut file = BufReader::new(File::open(file_info.path()).unwrap());
                    let mut file_contents = String::new();
                    let _size = file.read_to_string(&mut file_contents).unwrap();
                    data.layout_data.borrow_mut().layout = if let Ok(settings) =
                        LayoutSettings::from_json(file_contents.as_bytes())
                    {
                        Layout::from_settings(settings)
                    } else {
                        if let Ok(parsed_layout) = layout::parser::parse(file_contents.as_str()) {
                            parsed_layout
                        } else {
                            // TODO: Maybe dangerous
                            return;
                        }
                    };
                    data.config
                        .borrow_mut()
                        .set_layout_path(Some(file_info.path()));
                } else if command.is(CONTEXT_MENU_START_OR_SPLIT) {
                    data.timer.write().unwrap().split_or_start();
                } else if command.is(CONTEXT_MENU_RESET) {
                    // TODO: Ask user if they want to save best segments.
                    data.timer.write().unwrap().reset(true);
                } else if command.is(CONTEXT_MENU_UNDO_SPLIT) {
                    data.timer.write().unwrap().undo_split();
                } else if command.is(CONTEXT_MENU_SKIP_SPLIT) {
                    data.timer.write().unwrap().skip_split();
                } else if command.is(CONTEXT_MENU_TOGGLE_PAUSE) {
                    data.timer.write().unwrap().toggle_pause();
                } else if command.is(CONTEXT_MENU_UNDO_ALL_PAUSES) {
                    data.timer.write().unwrap().undo_all_pauses();
                } else if command.is(CONTEXT_MENU_TOGGLE_TIMING_METHOD) {
                    data.timer.write().unwrap().toggle_timing_method();
                } else if let Some(comparison) = command.get(CONTEXT_MENU_SET_COMPARISON) {
                    data.timer
                        .write()
                        .unwrap()
                        .set_current_comparison(comparison.as_str());
                } else if command.is(CONTEXT_MENU_EDIT_SETTINGS) {
                    data.hotkey_system.borrow_mut().deactivate();
                    let window =
                        WindowDesc::new(settings_editor::root_widget().lens(SettingsEditorLens))
                            .title("Settings")
                            .with_min_size((550.0, 400.0))
                            .window_size((550.0, 450.0))
                            .set_level(WindowLevel::Modal(ctx.window().clone()));
                    let window_id = window.id;
                    ctx.new_window(window);
                    data.settings_editor = Some(OpenWindow {
                        id: window_id,
                        state: settings_editor::State::new(data.hotkey_system.borrow().config()),
                    });
                }
            }
            _ => {}
        }
        self.inner.event(ctx, event, data, env)
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        _data: &MainState,
        _env: &Env,
    ) {
        if let LifeCycle::WidgetAdded = event {
            ctx.request_anim_frame();
            ctx.request_paint();
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &MainState, data: &MainState, env: &Env) {
        self.inner.update(ctx, old_data, data, env)
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &MainState,
        env: &Env,
    ) -> Size {
        self.inner.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut druid::PaintCtx, data: &MainState, env: &Env) {
        let mut layout_data = data.layout_data.borrow_mut();
        let layout_data = &mut *layout_data;

        if let Some(editor) = &data.layout_editor {
            editor
                .state
                .editor
                .borrow_mut()
                .as_mut()
                .unwrap()
                .update_layout_state(
                    &mut layout_data.layout_state,
                    &data.timer.read().unwrap().snapshot(),
                );
        } else {
            layout_data.layout.update_state(
                &mut layout_data.layout_state,
                &data.timer.read().unwrap().snapshot(),
            );
        }

        // let size = ctx.size();

        // if let Some((new_width, new_height)) = layout_data.scene_manager.update_scene(
        //     PietResourceAllocator,
        //     (size.width as f32, size.height as f32),
        //     &layout_data.layout_state,
        // ) {
        //     ctx.window()
        //         .set_size(Size::new(new_width as _, new_height as _));
        // }

        // software_renderer::render_scene(
        //     ctx,
        //     &mut self.bottom_image,
        //     &mut self.device,
        //     layout_data.scene_manager.scene(),
        // );

        if let Some((new_width, new_height)) = software_renderer::render_scene(
            ctx,
            &mut self.renderer,
            &layout_data.layout_state,
        ) {
            ctx.window()
                .set_size(Size::new(new_width as _, new_height as _));
        }
    }
}

struct DragWindowController {
    init_pos: Option<Point>,
}

impl DragWindowController {
    pub fn new() -> Self {
        DragWindowController { init_pos: None }
    }
}

impl<T, W: Widget<T>> Controller<T, W> for DragWindowController {
    fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        match event {
            Event::MouseDown(me) if me.buttons.has_left() => {
                ctx.set_active(true);
                self.init_pos = Some(me.window_pos)
            }
            Event::MouseMove(me) if ctx.is_active() => {
                if let Some(init_pos) = self.init_pos {
                    let window = ctx.window();
                    let within_window_change = me.window_pos.to_vec2() - init_pos.to_vec2();
                    let old_pos = window.get_position();
                    let new_pos = old_pos + within_window_change;
                    window.set_position(new_pos)
                }
            }
            Event::MouseUp(_me) if ctx.is_active() => {
                self.init_pos = None;
                ctx.set_active(false)
            }
            _ => (),
        }
        child.event(ctx, event, data, env)
    }
}

pub fn root_widget() -> impl Widget<MainState> {
    WithMenu::new(Flex::row()).controller(DragWindowController::new())
}

struct WindowManagement;

impl AppDelegate<MainState> for WindowManagement {
    fn window_removed(
        &mut self,
        id: WindowId,
        data: &mut MainState,
        env: &Env,
        ctx: &mut DelegateCtx,
    ) {
        if let Some(window) = &data.run_editor {
            if id == window.id {
                if window.state.closed_with_ok {
                    let run = window.state.editor.borrow_mut().take().unwrap().close();
                    data.timer
                        .write()
                        .unwrap()
                        .set_run(run)
                        .map_err(drop)
                        .unwrap();
                }
                data.run_editor = None;
                data.hotkey_system.borrow_mut().activate();
                return;
            }
        }

        if let Some(window) = &data.layout_editor {
            if id == window.id {
                if window.state.closed_with_ok {
                    let layout = window.state.editor.borrow_mut().take().unwrap().close();
                    data.layout_data.borrow_mut().layout = layout;
                }
                data.layout_editor = None;
                data.hotkey_system.borrow_mut().activate();
                return;
            }
        }

        if let Some(window) = &data.settings_editor {
            if id == window.id {
                if window.state.closed_with_ok {
                    let hotkey_config = window.state.editor.borrow_mut().take().unwrap();
                    data.hotkey_system.borrow_mut().set_config(hotkey_config);
                    data.config.borrow_mut().set_hotkeys(hotkey_config);
                }
                data.settings_editor = None;
                data.hotkey_system.borrow_mut().activate();
                return;
            }
        }
    }
}

pub fn launch(state: MainState, window: WindowDesc<MainState>) {
    AppLauncher::with_window(window)
        .configure_env(|env, _| {
            env.set(
                theme::SELECTED_TEXT_BACKGROUND_COLOR,
                SELECTED_TEXT_BACKGROUND_COLOR,
            );
            env.set(theme::BUTTON_LIGHT, BUTTON_TOP);
            env.set(theme::BUTTON_DARK, BUTTON_BOTTOM);
            env.set(theme::WINDOW_BACKGROUND_COLOR, BACKGROUND);
            env.set(theme::BORDER_DARK, BUTTON_BORDER);
            env.set(theme::BACKGROUND_LIGHT, TEXTBOX_BACKGROUND);
            env.set(theme::PRIMARY_LIGHT, PRIMARY_LIGHT);
            env.set(theme::BUTTON_BORDER_RADIUS, BUTTON_BORDER_RADIUS);
        })
        .delegate(WindowManagement)
        .launch(state)
        .unwrap();
}
