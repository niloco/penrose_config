use penrose::Error;
use std::process::{Child, Command, Stdio};

// use penrose::{
//     core::bindings::parse_keybindings_with_xmodmap,
//     core::WindowManager,
//     core::{bindings::keycodes_from_xmodmap, bindings::KeyCode},
//     x11rb::XcbConn,
// };
// use std::collections::HashMap;
// use once_cell::sync::OnceCell;

#[macro_use]
extern crate tracing;

// For spawning commands, capturing the output in /tmp/{cmd}.
// Can also spawn long-running processes (ie compositors) that will be killed on penrose exit.
pub struct SpawnHelper {
    procs: Vec<(String, Child)>,
}

impl SpawnHelper {
    pub fn new() -> Self {
        Self { procs: Vec::new() }
    }

    pub fn spawn_short(cmd: &str) -> penrose::Result<()> {
        let mut proc = Self::spawn(cmd)?;
        let status = proc.wait()?;
        if status.success() {
            info!("Command {} has run successfully", cmd);
            Ok(())
        } else {
            Err(Error::Custom(format!(
                "Command {} terminated with non-zero exit status: {}",
                cmd, status
            )))
        }
    }

    pub fn spawn_long(&mut self, cmd: &str) -> penrose::Result<()> {
        match Self::spawn(cmd) {
            Ok(proc) => {
                info!(
                    "Command {} spawned successfully with PID {}",
                    cmd,
                    proc.id()
                );
                self.procs.push((cmd.to_string(), proc));
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }

    fn spawn(cmd: &str) -> std::io::Result<Child> {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        let output_path = format!("/tmp/{}", parts[0]);
        let output_file = std::fs::File::create(&output_path)?;

        if parts.len() > 1 {
            Command::new(parts[0])
                .args(&parts[1..])
                .stdout(output_file.try_clone()?)
                .stderr(output_file)
                .spawn()
        } else {
            Command::new(parts[0])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
        }
    }
}

impl Drop for SpawnHelper {
    fn drop(&mut self) {
        for (cmd, proc) in self.procs.as_mut_slice() {
            match proc.kill() {
                Ok(_) => info!(
                    "Command {} with PID {} shutdown successfully",
                    cmd,
                    proc.id()
                ),
                Err(e) => error!(
                    "Command {} with PID {} could not be killed: {:?}",
                    cmd,
                    proc.id(),
                    e
                ),
            }
        }
    }
}

// boilerplate for more customizable bindings
// pub fn add_binding(
//     code: &str,
//     key_bindings: &mut HashMap<
//         KeyCode,
//         Box<dyn FnMut(&mut WindowManager<XcbConn>) -> Result<(), Error>>,
//     >,
//     callback: Box<dyn FnMut(&mut WindowManager<XcbConn>) -> Result<(), Error>>,
// ) -> penrose::Result<()> {
//     static CODES: OnceCell<HashMap<String, u8>> = OnceCell::new();
//     let codes =
//         CODES.get_or_init(|| keycodes_from_xmodmap().expect("make sure xmodmap is installed"));

//     match parse_keybindings_with_xmodmap(code.to_string(), &codes) {
//         // would be a lot cleaner with try_insert...
//         Some(key_code) => key_bindings
//             .insert(key_code, callback)
//             // None means empty, aka no dupes
//             .is_none()
//             .then(|| ())
//             .ok_or(Error::Custom(format!("{} has already been bound", code))),
//         None => Err(Error::Custom(format!(
//             "{} is not a valid key binding",
//             code
//         ))),
//     }
// }
