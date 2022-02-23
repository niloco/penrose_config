#[macro_use]
extern crate penrose;

use my_penrose::SpawnHelper;

use penrose::{
    core::{config::Config, helpers::index_selectors, layout, Layout},
    logging_error_handler,
    xcb::new_xcb_backed_window_manager,
    Backward, Forward, Less, More, PenroseError,
};

// Spawning background/setup stuff
// If something fails, don't start the WM
fn setup() -> penrose::Result<SpawnHelper> {
    // Commands that run to completion
    const WALLPAPER: &str = "feh --bg-fill /home/niloco/pics/pawel-blue.jpg";
    SpawnHelper::spawn_short(WALLPAPER)?;

    // Long running stuff (like picom)
    const COMPOSITOR: &str = "picom";
    let mut proc_handles = SpawnHelper::new();
    proc_handles.spawn_long(COMPOSITOR)?;

    Ok(proc_handles)
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

fn main() -> penrose::Result<()> {
    tracing_subscriber::fmt::fmt()
        .pretty()
        .with_env_filter("trace")
        .try_init()
        .map_err(|e| {
            let raw_err = format!("{:?}", e);
            PenroseError::Raw(raw_err)
        })?;

    // Aesthetic stuff
    const FOCUSED_BORDER_COLOR: &str = "#bb9af7ff";
    const UNFOCUSED_BORDER_COLOR: &str = "#a9b1d6ff";
    const BORDER_SIZE: u32 = 4;
    const BAR_HEIGHT: u32 = 0;

    // Build config
    let layouts = layouts();
    let mut config_builder = Config::default().builder();
    let config = config_builder
        .border_px(BORDER_SIZE)
        .focused_border(FOCUSED_BORDER_COLOR)
        .expect("focused border failed parsing")
        .unfocused_border(UNFOCUSED_BORDER_COLOR)
        .expect("unfocused border failed parsing")
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

    let key_bindings = gen_keybindings! {
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
        "M-C-j" => run_internal!(cycle_workspace, Forward);
        "M-C-k" => run_internal!(cycle_workspace, Backward);

        // Layout management
        "M-m" => run_internal!(cycle_layout, Forward);
        "M-period" => run_internal!(update_max_main, More);
        "M-comma" => run_internal!(update_max_main, Less);
        "M-l" => run_internal!(update_main_ratio, More);
        "M-h" => run_internal!(update_main_ratio, Less);

        map: { "1", "2", "3", "4", "5", "6", "7", "8", "9" } to index_selectors(9) => {
            "M-{}" => focus_workspace (REF);
            "M-S-{}" => client_to_workspace (REF);
        };
    };

    let mut wm = new_xcb_backed_window_manager(config, vec![], logging_error_handler())?;
    let _procs = setup()?;
    wm.grab_keys_and_run(key_bindings, map! {})
}
