use std::{
    collections::HashMap,
    process::{Child, Command, Stdio},
};

use once_cell::sync::OnceCell;
use penrose::{
    core::{bindings::KeyCode, helpers::keycodes_from_xmodmap},
    xcb::helpers::parse_key_binding,
    PenroseError, WindowManager, XcbConnection,
};

// for spawning long running programs
pub struct ProcessHolder {
    procs: Vec<Child>,
}

impl ProcessHolder {
    pub fn new() -> Self {
        Self { procs: Vec::new() }
    }

    pub fn spawn_long<S: Into<String>>(&mut self, cmd: S) -> penrose::Result<()> {
        let s = cmd.into();
        let parts: Vec<&str> = s.split_whitespace().collect();
        let result = if parts.len() > 1 {
            Command::new(parts[0])
                .args(&parts[1..])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
        } else {
            Command::new(parts[0])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
        };

        match result {
            Ok(proc) => {
                self.procs.push(proc);
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }

    pub fn spawn_long_with_args<S: Into<String>>(
        &mut self,
        cmd: S,
        args: &[&str],
    ) -> penrose::Result<()> {
        let result = Command::new(cmd.into())
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        match result {
            Ok(proc) => {
                self.procs.push(proc);
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }
}

impl Drop for ProcessHolder {
    fn drop(&mut self) {
        for child in self.procs.as_mut_slice() {
            if let Err(e) = child.kill() {
                println!("{:?}", e)
            }
        }
    }
}

// boilerplate for more customizable bindings
pub fn add_binding(
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
