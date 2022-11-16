#[macro_use]
extern crate penrose;

use std::collections::HashMap;

use my_penrose::SpawnHelper;

use penrose::{
    builtin::actions::{exit, modify_with, send_layout_message, spawn},
    builtin::layout::{
        messages::{ExpandMain, IncMain, ShrinkMain},
        transformers::Gaps,
        MainAndStack, Monocle,
    },
    core::{bindings::parse_keybindings_with_xmodmap, layout::LayoutStack, Config, WindowManager},
    x11rb::RustConn,
    Color, Error,
};

// Spawning background/setup stuff
// If something fails, don't start the WM
fn setup() -> penrose::Result<SpawnHelper> {
    // Commands that run to completion
    const WALLPAPER: &str = "feh --bg-fill /home/niloco/pics/pawel-blue.jpg";
    SpawnHelper::spawn_short(WALLPAPER)?;

    // Long running stuff (compositor, etc)
    let mut proc_handles = SpawnHelper::new();

    const COMPOSITOR: &str = "picom";
    proc_handles.spawn_long(COMPOSITOR)?;

    // notifications
    const DUNST: &str = "dunst";
    const BATMON: &str = "batmon";
    proc_handles.spawn_long(DUNST)?;
    proc_handles.spawn_long(BATMON)?;

    Ok(proc_handles)
}

// Defining my layouts
fn layouts() -> LayoutStack {
    stack![
        Gaps::wrap(MainAndStack::side(1, 0.6, 0.05), 4, 4),
        Monocle::boxed()
    ]
}

fn main() -> penrose::Result<()> {
    tracing_subscriber::fmt::fmt()
        .pretty()
        .with_env_filter("info")
        .try_init()
        .map_err(|e| {
            let raw_err = format!("{:?}", e);
            Error::Custom(raw_err)
        })?;

    // Aesthetic stuff
    const FOCUSED_BORDER_COLOR: u32 = 0xbb9af7ff;
    const UNFOCUSED_BORDER_COLOR: u32 = 0xa9b1d6ff;
    const BORDER_SIZE: u32 = 4;

    let config = Config {
        normal_border: Color::new_from_hex(UNFOCUSED_BORDER_COLOR),
        focused_border: Color::new_from_hex(FOCUSED_BORDER_COLOR),
        border_width: BORDER_SIZE,
        focus_follow_mouse: true,
        default_layouts: layouts(),
        tags: ["1", "2", "3", "4", "5", "6", "7", "8", "9"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        floating_classes: Vec::new(),
        startup_hook: None,
        event_hook: None,
        manage_hook: None,
        refresh_hook: None,
    };

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

    let mut key_bindings = map! {
        map_keys: |k: &str| k.to_string();

        // Program launcher
        "M-space" => spawn(LAUNCHER),

        // Terminal
        "M-Return" => spawn(TERMINAL),

        // xf86 things
        "XF86AudioRaiseVolume" => spawn(AUDIO_RAISE),
        "XF86AudioLowerVolume" => spawn(AUDIO_LOWER),
        "XF86AudioMute" => spawn(AUDIO_MUTE),
        "XF86AudioMicMute" => spawn(MIC_MUTE),
        "XF86MonBrightnessUp" => spawn(BACKLIGHT_RAISE),
        "XF86MonBrightnessDown" => spawn(BACKLIGHT_LOWER),

        // Session lock
        "M-u" => spawn(LOCK),

        // Exit Penrose (important to remember this one!)
        "M-S-e" => exit(),

        // client management
        "M-j" => modify_with(|cs| cs.focus_down()),
        "M-k" => modify_with(|cs| cs.focus_up()),
        "M-S-j" => modify_with(|cs| cs.swap_down()),
        "M-S-k" => modify_with(|cs| cs.swap_up()),
        "M-w" => modify_with(|cs| cs.kill_focused()),

        // workspace management
        // "M-Tab" => modify_with(|cs| , toggle_workspace),
        // "M-C-j" => modify_with(|cs| , cycle_workspace, Forward),
        // "M-C-k" => modify_with(|cs| , cycle_workspace, Backward),

        // Layout management
        "M-m" => modify_with(|cs| cs.next_layout()),
        "M-period" => send_layout_message( || IncMain(1)),
        "M-comma" => send_layout_message( || IncMain(-1)),
        "M-l" => send_layout_message( || ExpandMain),
        "M-h" => send_layout_message( || ShrinkMain),
    };

    for tag in &["1", "2", "3", "4", "5", "6", "7", "8", "9"] {
        key_bindings.extend([
            (
                format!("M-{tag}"),
                modify_with(move |client_set| client_set.focus_tag(tag)),
            ),
            (
                format!("M-S-{tag}"),
                modify_with(move |client_set| client_set.move_focused_to_tag(tag)),
            ),
        ]);
    }

    let key_bindings = parse_keybindings_with_xmodmap(key_bindings)?;
    let conn = RustConn::new()?;
    let wm = WindowManager::new(config, key_bindings, HashMap::new(), conn)?;

    match setup() {
        Ok(_procs) => wm.run(),
        Err(e) => Err(e),
    }
}
