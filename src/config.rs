use directories::ProjectDirs;
use druid::WindowDesc;
use livesplit_core::{
    layout::{self, Layout, LayoutSettings},
    run::{parser::composite, saver::livesplit::save_timer},
    HotkeyConfig, HotkeySystem, Run, Segment, Timer, TimingMethod,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, create_dir_all, File},
    io::{BufReader, BufWriter, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use crate::{timer_form, MainState};

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(default)]
    general: General,
    #[serde(default)]
    log: Log,
    #[serde(default)]
    window: Window,
    #[serde(default)]
    hotkeys: HotkeyConfig,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
struct General {
    splits: Option<PathBuf>,
    layout: Option<PathBuf>,
    timing_method: Option<TimingMethod>,
    comparison: Option<String>,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
struct Log {
    #[serde(default)]
    enable: bool,
    level: Option<log::LevelFilter>,
    #[serde(default)]
    clear: bool,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(default)]
struct Window {
    width: f64,
    height: f64,
}

impl Default for Window {
    fn default() -> Window {
        Self {
            width: 300.0,
            height: 500.0,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        Self::config_path()
            .and_then(Self::parse)
            .unwrap_or_default()
    }

    pub fn save(&self) -> Option<()> {
        let path = Self::config_path()?;
        create_dir_all(path.parent()?).ok()?;
        self.serialize(path)
    }

    pub fn config_path() -> Option<PathBuf> {
        Self::path("config.yml")
    }

    pub fn path(file: &str) -> Option<PathBuf> {
        Some(
            ProjectDirs::from("org", "LiveSplit", "LiveSplit One")?
                .data_local_dir()
                .join(file),
        )
    }

    pub fn parse(path: impl AsRef<Path>) -> Option<Self> {
        let buf = fs::read(path).ok()?;
        serde_yaml::from_slice(&buf).ok()
    }

    pub fn serialize(&self, path: impl AsRef<Path>) -> Option<()> {
        let buf = serde_yaml::to_vec(self).ok()?;
        fs::write(path, &buf).ok()
    }

    pub fn parse_run(&self) -> Option<Run> {
        let path = self.general.splits.clone()?;
        let file = BufReader::new(File::open(&path).ok()?);
        let mut run = composite::parse(file, Some(path), true).ok()?.run;
        run.fix_splits();
        Some(run)
    }

    pub fn parse_run_or_default(&self) -> Run {
        self.parse_run().unwrap_or_else(|| {
            let mut run = Run::new();
            run.push_segment(Segment::new("Time"));
            run
        })
    }

    pub fn is_game_time(&self) -> bool {
        self.general.timing_method == Some(TimingMethod::GameTime)
    }

    pub fn parse_layout(&self) -> Option<Layout> {
        // TODO: Use these for open splits in the right click menu.
        let path = self.general.layout.as_ref()?;
        let mut file = BufReader::new(File::open(path).ok()?);
        if let Ok(settings) = LayoutSettings::from_json(&mut file) {
            return Some(Layout::from_settings(settings));
        }
        file.seek(SeekFrom::Start(0)).ok()?;
        layout::parser::parse(file).ok()
    }

    pub fn parse_layout_or_default(&self) -> Layout {
        self.parse_layout().unwrap_or_else(Layout::default_layout)
    }

    // pub fn set_splits_path(&mut self, path: PathBuf) {
    //     self.general.splits = Some(path);
    // }

    // TODO: Just directly construct the HotkeySystem from the config.
    pub fn configure_hotkeys(&self, hotkeys: &mut HotkeySystem) {
        hotkeys.set_config(self.hotkeys).ok();
    }

    pub fn configure_timer(&self, timer: &mut Timer) {
        if self.is_game_time() {
            timer.set_current_timing_method(TimingMethod::GameTime);
        }
        if let Some(comparison) = &self.general.comparison {
            timer.set_current_comparison(comparison).ok();
        }
    }

    pub fn save_splits(&self, timer: &Timer) {
        if let Some(path) = &self.general.splits {
            // FIXME: Don't ignore not being able to save.
            if let Ok(file) = File::create(path) {
                save_timer(timer, BufWriter::new(file)).ok();
            }
        }
    }

    pub fn set_hotkeys(&mut self, hotkeys: HotkeyConfig) {
        self.hotkeys = hotkeys;
        self.save();
    }

    pub fn set_splits_path(&mut self, path: Option<&Path>) {
        self.general.splits = path.map(|path| path.to_path_buf());
        self.save();
    }

    pub fn set_layout_path(&mut self, path: Option<&Path>) {
        self.general.layout = path.map(|path| path.to_path_buf());
        self.save();
    }

    pub fn setup_logging(&self) -> Option<()> {
        if self.log.enable {
            let path = Self::path("log.txt")?;
            create_dir_all(path.parent()?).ok()?;

            let log_file = fs::OpenOptions::new()
                .create(true)
                .write(true)
                .append(!self.log.clear)
                .truncate(self.log.clear)
                .open(&path)
                .ok()?;

            fern::Dispatch::new()
                .format(|out, message, record| {
                    out.finish(format_args!(
                        "{}[{}][{}] {}",
                        chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                        record.target(),
                        record.level(),
                        message
                    ))
                })
                .level(self.log.level.unwrap_or(log::LevelFilter::Warn))
                .chain(log_file)
                .apply()
                .ok()?;

            #[cfg(not(debug_assertions))]
            {
                std::panic::set_hook(Box::new(|panic_info| {
                    log::error!(target: "PANIC", "{}\n{:?}", panic_info, backtrace::Backtrace::new());
                }));
            }
        }
        Some(())
    }

    pub fn build_window(&self) -> WindowDesc<MainState> {
        WindowDesc::new(timer_form::root_widget())
            .title("LiveSplit One")
            .with_min_size((50.0, 50.0))
            .window_size((self.window.width, self.window.height))
            .show_titlebar(false)
            .transparent(true)
            .topmost(true)
    }
}
