#[macro_use]
extern crate penrose;

use std::collections::HashMap;

use once_cell::sync::OnceCell;
use penrose::{
    core::{
        bindings::KeyCode,
        config::Config,
        helpers::{index_selectors, keycodes_from_xmodmap, spawn},
        layout, Layout,
    },
    logging_error_handler,
    xcb::{helpers::parse_key_binding, new_xcb_backed_window_manager},
    Backward, Forward, Less, More, PenroseError, WindowManager, XcbConnection,
};

use simplelog::{LevelFilter, SimpleLogger};

// Spawning background/setup stuff
// If something fails, don't start the WM
fn setup() -> penrose::Result<()> {
    // Commands that run at startup
    const WALLPAPER: &str = "feh --bg-fill /home/niloco/pics/pawel-blue.jpg";

    spawn(WALLPAPER)?;
    Ok(())
}

// Defining my layouts
fn layouts() -> Vec<Layout> {
    vec![
        layout::Layout::new(
            "stack",
            layout::LayoutConf::default(),
            layout::side_stack,
            1,
            0.6,
        ),
        layout::Layout::new(
            "mono",
            layout::LayoutConf {
                floating: false,
                gapless: true,
                follow_focus: true,
                allow_wrapping: true,
            },
            layout::monocle,
            1,
            0.6,
        ),
    ]
}

#[derive(Clone, Copy)]
enum KbState {
    On,
    Off,
}

impl KbState {
    pub fn new() -> Self {
        Self::Off
    }

    pub fn toggle(state: Self) -> penrose::Result<Self> {
        const ON_COMMAND: &str = "setxkbmap -model pc105 -layout us -option ctrl:swapcaps";
        const OFF_COMMAND: &str = "setxkbmap -model pc105 -layout us -option";

        match state {
            Self::On => {
                spawn(OFF_COMMAND)?;
                Ok(Self::Off)
            }
            Self::Off => {
                spawn(ON_COMMAND)?;
                Ok(Self::On)
            }
        }
    }
}

struct KbToggle {
    state: KbState,
}

impl KbToggle {
    pub fn new() -> Self {
        Self {
            state: KbState::new(),
        }
    }

    // toggles the returns new state
    pub fn toggle(&mut self) -> penrose::Result<()> {
        self.state = KbState::toggle(self.state)?;
        Ok(())
    }
}

// boilerplate for more customizable bindings
fn add_binding(
    code: &str,
    key_bindings: &mut HashMap<
        KeyCode,
        Box<dyn FnMut(&mut WindowManager<XcbConnection>) -> Result<(), PenroseError>>,
    >,
    callback: Box<dyn FnMut(&mut WindowManager<XcbConnection>) -> Result<(), PenroseError>>,
) -> penrose::Result<()> {
    static CODES: OnceCell<HashMap<String, u8>> = OnceCell::new();
    let codes = CODES.get_or_init(|| keycodes_from_xmodmap());

    match parse_key_binding(code, &codes) {
        // would be a lot cleaner with try_insert...
        Some(key_code) => key_bindings
            .insert(key_code, callback)
            // None means empty, aka no dupes
            .is_none()
            .then(|| ())
            .ok_or(PenroseError::Raw(format!(
                "{} has already been bound",
                code
            ))),
        None => Err(PenroseError::Raw(format!(
            "{} is not a valid key binding",
            code
        ))),
    }
}

fn main() -> penrose::Result<()> {
    // Initialise the logger (use LevelFilter::Debug to enable debug logging)
    SimpleLogger::init(LevelFilter::Info, simplelog::Config::default()).map_err(|e| {
        let msg = format!("unable to set log level: {}", e);
        PenroseError::Raw(msg)
    })?;

    // Aesthetic stuff
    const FOCUSED_BORDER_COLOR: u32 = 0xbb9af7;
    const UNFOCUSED_BORDER_COLOR: u32 = 0xa9b1d6;
    const BORDER_SIZE: u32 = 4;
    const BAR_HEIGHT: u32 = 0;

    // Build config
    let layouts = layouts();
    let mut config_builder = Config::default().builder();
    let config = config_builder
        .border_px(BORDER_SIZE)
        .focused_border(FOCUSED_BORDER_COLOR)
        .unfocused_border(UNFOCUSED_BORDER_COLOR)
        .layouts(layouts)
        .bar_height(BAR_HEIGHT)
        .build()
        .map_err(|s| PenroseError::Raw(s))?;

    // Commands for runtime
    const TERMINAL: &str = "alacritty";
    const LAUNCHER: &str = "rofi -no-lazy-grab -show run";
    const LOCK: &str = "xsecurelock";

    // xf86 commands
    const AUDIO_RAISE: &str = "pamixer -i 5";
    const AUDIO_LOWER: &str = "pamixer -d 5";
    const AUDIO_MUTE: &str = "pamixer -t";
    const MIC_MUTE: &str = "pamixer --source 1 -t";
    const BACKLIGHT_RAISE: &str = "brightnessctl set +2%";
    const BACKLIGHT_LOWER: &str = "brightnessctl set 2%-";

    let mut toggler = KbToggle::new();

    let mut key_bindings = gen_keybindings! {
        // Program launcher
        "M-space" => run_external!(LAUNCHER);

        // Terminal
        "M-Return" => run_external!(TERMINAL);

        // xf86 things
        "XF86AudioRaiseVolume" => run_external!(AUDIO_RAISE);
        "XF86AudioLowerVolume" => run_external!(AUDIO_LOWER);
        "XF86AudioMute" => run_external!(AUDIO_MUTE);
        "XF86AudioMicMute" => run_external!(MIC_MUTE);
        "XF86MonBrightnessUp" => run_external!(BACKLIGHT_RAISE);
        "XF86MonBrightnessDown" => run_external!(BACKLIGHT_LOWER);

        // Session lock
        "M-u" => run_external!(LOCK);

        // Exit Penrose (important to remember this one!)
        "M-S-e" => run_internal!(exit);

        // client management
        "M-j" => run_internal!(cycle_client, Forward);
        "M-k" => run_internal!(cycle_client, Backward);
        "M-S-j" => run_internal!(drag_client, Forward);
        "M-S-k" => run_internal!(drag_client, Backward);
        "M-w" => run_internal!(kill_client);

        // workspace management
        "M-Tab" => run_internal!(toggle_workspace);
        "M-semicolon" => run_internal!(cycle_workspace, Forward);
        "M-apostrophe" => run_internal!(cycle_workspace, Backward);

        // Layout management
        "M-m" => run_internal!(cycle_layout, Forward);
        "M-period" => run_internal!(update_max_main, More);
        "M-comma" => run_internal!(update_max_main, Less);
        "M-l" => run_internal!(update_main_ratio, More);
        "M-h" => run_internal!(update_main_ratio, Less);

        refmap [ config.ws_range() ] in {
            "M-{}" => focus_workspace [ index_selectors(config.workspaces().len()) ];
            "M-S-{}" => client_to_workspace [ index_selectors(config.workspaces().len()) ];
        };
    };

    // hand-rolled bindings

    // swaps capslock and left control
    add_binding(
        "M-t",
        &mut key_bindings,
        Box::new(move |_| toggler.toggle()),
    )?;

    let mut wm = new_xcb_backed_window_manager(config, vec![], logging_error_handler())?;
    setup()?;
    wm.grab_keys_and_run(key_bindings, map! {})
}
