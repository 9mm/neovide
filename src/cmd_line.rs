use std::{iter, mem};

use crate::{dimensions::Dimensions, frame::Frame, settings::*};

use anyhow::Result;
use clap::{builder::FalseyValueParser, ArgAction, Parser};

#[cfg(target_os = "windows")]
pub const SRGB_DEFAULT: &str = "1";
#[cfg(not(target_os = "windows"))]
pub const SRGB_DEFAULT: &str = "0";

#[derive(Clone, Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct CmdLineSettings {
    /// Files to open (plainly appended to NeoVim args)
    #[arg(
        num_args = ..,
        action = ArgAction::Append,
    )]
    pub files_to_open: Vec<String>,

    /// Arguments to pass down to NeoVim without interpreting them
    #[arg(
        num_args = ..,
        action = ArgAction::Append,
        last = true,
        allow_hyphen_values = true
    )]
    pub neovim_args: Vec<String>,

    /// If to enable logging to a file in the current directory
    #[arg(long = "log")]
    pub log_to_file: bool,

    /// Connect to the named pipe or socket at ADDRESS
    #[arg(long, alias = "remote-tcp", value_name = "ADDRESS")]
    pub server: Option<String>,

    /// Run NeoVim in WSL rather than on the host
    #[arg(long, env = "NEOVIDE_WSL")]
    pub wsl: bool,

    /// Which window decorations to use (do note that the window might not be resizable
    /// if this is "none")
    #[arg(long, env = "NEOVIDE_FRAME", default_value_t)]
    pub frame: Frame,

    /// Disable the Multigrid extension (disables smooth scrolling, window animations, and floating blur)
    #[arg(long = "no-multigrid", env = "NEOVIDE_NO_MULTIGRID", value_parser = FalseyValueParser::new())]
    pub no_multi_grid: bool,

    /// Instead of spawning a child process and leaking it, be "blocking" and let the shell persist
    /// as parent process
    #[arg(long = "no-fork")]
    pub no_fork: bool,

    /// Render every frame, takes more power and CPU time but possibly helps with frame timing
    /// issues
    #[arg(long = "no-idle", env = "NEOVIDE_IDLE", action = ArgAction::SetFalse, value_parser = FalseyValueParser::new())]
    pub idle: bool,

    /// Disable opening multiple files supplied in tabs (they're still buffers)
    #[arg(long = "no-tabs")]
    pub no_tabs: bool,

    /// Request sRGB when initializing the window, may help with GPUs with weird pixel
    /// formats. Default on Windows.
    #[arg(long = "srgb", env = "NEOVIDE_SRGB", action = ArgAction::SetTrue, default_value = SRGB_DEFAULT, value_parser = FalseyValueParser::new())]
    pub srgb: bool,

    /// Do not request sRGB when initializing the window, may help with GPUs with weird pixel
    /// formats. Default on Linux and macOs.
    #[arg(long = "no-srgb", action = ArgAction::SetTrue, value_parser = FalseyValueParser::new())]
    _no_srgb: bool,

    /// Request VSync on the window [DEFAULT]
    #[arg(long = "vsync", env = "NEOVIDE_VSYNC", action = ArgAction::SetTrue, default_value = "1", value_parser = FalseyValueParser::new())]
    pub vsync: bool,

    /// Do not try to request VSync on the window
    #[arg(long = "no-vsync", action = ArgAction::SetTrue, value_parser = FalseyValueParser::new())]
    _no_vsync: bool,

    /// Which NeoVim binary to invoke headlessly instead of `nvim` found on $PATH
    #[arg(long = "neovim-bin", env = "NEOVIM_BIN")]
    pub neovim_bin: Option<String>,

    /// The app ID to show to the compositor (Wayland only, useful for setting WM rules)
    #[arg(
        long = "wayland_app_id",
        env = "NEOVIDE_APP_ID",
        default_value = "neovide"
    )]
    pub wayland_app_id: String,

    /// The class part of the X11 WM_CLASS property (X only, useful for setting WM rules)
    #[arg(
        long = "x11-wm-class",
        env = "NEOVIDE_WM_CLASS",
        default_value = "neovide"
    )]
    pub x11_wm_class: String,

    /// The instance part of the X11 WM_CLASS property (X only, useful for setting WM rules)
    #[arg(
        long = "x11-wm-class-instance",
        env = "NEOVIDE_WM_CLASS_INSTANCE",
        default_value = "neovide"
    )]
    pub x11_wm_class_instance: String,

    #[command(flatten)]
    pub geometry: GeometryArgs,
}

// geometry, size and maximized are mutually exclusive
#[derive(Clone, Debug, Args, PartialEq)]
#[group(required = false, multiple = false)]
pub struct GeometryArgs {
    /// The initial grid size of the window [<columns>x<lines>]. Defaults to columns/lines from init.vim/lua if no value is given.
    /// If --grid is not set then it's inferred from the window size
    #[arg(long)]
    pub grid: Option<Option<Dimensions>>,

    /// The size of the window in pixels.
    #[arg(long)]
    pub size: Option<Dimensions>,

    /// Maximize the window on startup (not equivalent to fullscreen)
    #[arg(long, env = "NEOVIDE_MAXIMIZED", value_parser = FalseyValueParser::new())]
    pub maximized: bool,
}

impl Default for CmdLineSettings {
    fn default() -> Self {
        Self::parse_from(iter::empty::<String>())
    }
}

pub fn handle_command_line_arguments(args: Vec<String>) -> Result<()> {
    let mut cmdline = CmdLineSettings::try_parse_from(args)?;

    // The neovim_args in cmdline are unprocessed, actually add options to it
    let maybe_tab_flag = (!cmdline.no_tabs).then(|| "-p".to_string());

    cmdline.neovim_args = maybe_tab_flag
        .into_iter()
        .chain(mem::take(&mut cmdline.files_to_open))
        .chain(cmdline.neovim_args)
        .collect();

    if cmdline._no_vsync {
        cmdline.vsync = false;
    }

    if cmdline._no_srgb {
        cmdline.srgb = false;
    }

    SETTINGS.set::<CmdLineSettings>(&cmdline);
    Ok(())
}

#[cfg(test)]
#[allow(clippy::bool_assert_comparison)] // useful here since the explicit true/false comparison matters
mod tests {
    use scoped_env::ScopedEnv;

    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_neovim_passthrough() {
        let args: Vec<String> = vec!["neovide", "--no-tabs", "--", "--clean"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        handle_command_line_arguments(args).expect("Could not parse arguments");
        assert_eq!(
            SETTINGS.get::<CmdLineSettings>().neovim_args,
            vec!["--clean"]
        );
    }

    #[test]
    #[serial]
    fn test_files_to_open() {
        let args: Vec<String> = vec!["neovide", "./foo.txt", "--no-tabs", "./bar.md"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        handle_command_line_arguments(args).expect("Could not parse arguments");
        assert_eq!(
            SETTINGS.get::<CmdLineSettings>().neovim_args,
            vec!["./foo.txt", "./bar.md"]
        );
    }

    #[test]
    #[serial]
    fn test_files_to_open_with_passthrough() {
        let args: Vec<String> = vec![
            "neovide",
            "--no-tabs",
            "./foo.txt",
            "./bar.md",
            "--",
            "--clean",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();

        handle_command_line_arguments(args).expect("Could not parse arguments");
        assert_eq!(
            SETTINGS.get::<CmdLineSettings>().neovim_args,
            vec!["./foo.txt", "./bar.md", "--clean"]
        );
    }

    #[test]
    #[serial]
    fn test_files_to_open_with_flag() {
        let args: Vec<String> = vec!["neovide", "./foo.txt", "./bar.md", "--grid=42x24"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        handle_command_line_arguments(args).expect("Could not parse arguments");
        assert_eq!(
            SETTINGS.get::<CmdLineSettings>().neovim_args,
            vec!["-p", "./foo.txt", "./bar.md"]
        );

        assert_eq!(
            SETTINGS.get::<CmdLineSettings>().geometry.grid,
            Some(Some(Dimensions {
                width: 42,
                height: 24
            })),
        );
    }

    #[test]
    #[serial]
    fn test_grid() {
        let args: Vec<String> = vec!["neovide", "--grid=42x24"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        handle_command_line_arguments(args).expect("Could not parse arguments");
        assert_eq!(
            SETTINGS.get::<CmdLineSettings>().geometry.grid,
            Some(Some(Dimensions {
                width: 42,
                height: 24
            })),
        );
    }

    #[test]
    #[serial]
    fn test_size() {
        let args: Vec<String> = vec!["neovide", "--size=420x240"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        handle_command_line_arguments(args).expect("Could not parse arguments");
        assert_eq!(
            SETTINGS.get::<CmdLineSettings>().geometry.size,
            Some(Dimensions {
                width: 420,
                height: 240,
            }),
        );
    }

    #[test]
    #[serial]
    fn test_log_to_file() {
        let args: Vec<String> = vec!["neovide", "--log"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        handle_command_line_arguments(args).expect("Could not parse arguments");
        assert!(SETTINGS.get::<CmdLineSettings>().log_to_file);
    }

    #[test]
    #[serial]
    fn test_frameless_flag() {
        let args: Vec<String> = vec!["neovide", "--frame=full"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        handle_command_line_arguments(args).expect("Could not parse arguments");
        assert_eq!(SETTINGS.get::<CmdLineSettings>().frame, Frame::Full);
    }

    #[test]
    #[serial]
    fn test_frameless_environment_variable() {
        let args: Vec<String> = vec!["neovide"].iter().map(|s| s.to_string()).collect();

        let _env = ScopedEnv::set("NEOVIDE_FRAME", "none");
        handle_command_line_arguments(args).expect("Could not parse arguments");
        assert_eq!(SETTINGS.get::<CmdLineSettings>().frame, Frame::None);
    }

    #[test]
    #[serial]
    fn test_neovim_bin_arg() {
        let args: Vec<String> = vec!["neovide", "--neovim-bin", "foo"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        handle_command_line_arguments(args).expect("Could not parse arguments");
        assert_eq!(
            SETTINGS.get::<CmdLineSettings>().neovim_bin,
            Some("foo".to_owned())
        );
    }

    #[test]
    #[serial]
    fn test_neovim_bin_environment_variable() {
        let args: Vec<String> = vec!["neovide"].iter().map(|s| s.to_string()).collect();

        let _env = ScopedEnv::set("NEOVIM_BIN", "foo");
        handle_command_line_arguments(args).expect("Could not parse arguments");
        assert_eq!(
            SETTINGS.get::<CmdLineSettings>().neovim_bin,
            Some("foo".to_owned())
        );
    }

    #[test]
    #[serial]
    fn test_srgb_default() {
        let args: Vec<String> = vec!["neovide"].iter().map(|s| s.to_string()).collect();

        handle_command_line_arguments(args).expect("Could not parse arguments");
        #[cfg(target_os = "windows")]
        let default_value = true;
        #[cfg(not(target_os = "windows"))]
        let default_value = false;
        assert_eq!(SETTINGS.get::<CmdLineSettings>().srgb, default_value);
    }

    #[test]
    #[serial]
    fn test_srgb() {
        let args: Vec<String> = vec!["neovide", "--srgb"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        handle_command_line_arguments(args).expect("Could not parse arguments");
        assert_eq!(SETTINGS.get::<CmdLineSettings>().srgb, true);
    }

    #[test]
    #[serial]
    fn test_nosrgb() {
        let args: Vec<String> = vec!["neovide", "--no-srgb"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        handle_command_line_arguments(args).expect("Could not parse arguments");
        assert_eq!(SETTINGS.get::<CmdLineSettings>().srgb, false);
    }

    #[test]
    #[serial]
    fn test_no_srgb_environment() {
        let args: Vec<String> = vec!["neovide"].iter().map(|s| s.to_string()).collect();

        let _env = ScopedEnv::set("NEOVIDE_SRGB", "0");
        handle_command_line_arguments(args).expect("Could not parse arguments");
        assert_eq!(SETTINGS.get::<CmdLineSettings>().srgb, false);
    }

    #[test]
    #[serial]
    fn test_override_srgb_environment() {
        let args: Vec<String> = vec!["neovide", "--no-srgb"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let _env = ScopedEnv::set("NEOVIDE_SRGB", "1");
        handle_command_line_arguments(args).expect("Could not parse arguments");
        assert_eq!(SETTINGS.get::<CmdLineSettings>().srgb, false);
    }

    #[test]
    #[serial]
    fn test_override_nosrgb_environment() {
        let args: Vec<String> = vec!["neovide", "--srgb"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let _env = ScopedEnv::set("NEOVIDE_SRGB", "0");
        handle_command_line_arguments(args).expect("Could not parse arguments");
        assert_eq!(SETTINGS.get::<CmdLineSettings>().srgb, true,);
    }

    #[test]
    #[serial]
    fn test_vsync_default() {
        let args: Vec<String> = vec!["neovide"].iter().map(|s| s.to_string()).collect();

        handle_command_line_arguments(args).expect("Could not parse arguments");
        assert_eq!(SETTINGS.get::<CmdLineSettings>().vsync, true);
    }

    #[test]
    #[serial]
    fn test_vsync() {
        let args: Vec<String> = vec!["neovide", "--vsync"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        handle_command_line_arguments(args).expect("Could not parse arguments");
        assert_eq!(SETTINGS.get::<CmdLineSettings>().vsync, true);
    }

    #[test]
    #[serial]
    fn test_novsync() {
        let args: Vec<String> = vec!["neovide", "--no-vsync"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        handle_command_line_arguments(args).expect("Could not parse arguments");
        assert_eq!(SETTINGS.get::<CmdLineSettings>().vsync, false);
    }

    #[test]
    #[serial]
    fn test_no_vsync_environment() {
        let args: Vec<String> = vec!["neovide"].iter().map(|s| s.to_string()).collect();

        let _env = ScopedEnv::set("NEOVIDE_VSYNC", "0");
        handle_command_line_arguments(args).expect("Could not parse arguments");
        assert_eq!(SETTINGS.get::<CmdLineSettings>().vsync, false);
    }

    #[test]
    #[serial]
    fn test_override_vsync_environment() {
        let args: Vec<String> = vec!["neovide", "--no-vsync"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let _env = ScopedEnv::set("NEOVIDE_VSYNC", "1");
        handle_command_line_arguments(args).expect("Could not parse arguments");
        assert_eq!(SETTINGS.get::<CmdLineSettings>().vsync, false);
    }

    #[test]
    #[serial]
    fn test_override_novsync_environment() {
        let args: Vec<String> = vec!["neovide", "--vsync"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let _env = ScopedEnv::set("NEOVIDE_VSYNC", "0");
        handle_command_line_arguments(args).expect("Could not parse arguments");
        assert_eq!(SETTINGS.get::<CmdLineSettings>().vsync, true,);
    }
}
