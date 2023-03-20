// TODO: module comments ? (use //!, with short title description)
// TODO: decent source documentation for most things

#[path = "../config.rs"]
mod config;

mod error;
mod env;
mod recover;
mod input_pw;
mod serial;
mod find;
mod tui;
mod output;
mod util;

use error::{Error, Result};
use env::prelude::*;

use util::{
    file,
    user_io,
    proc,
};

use util::{
    file::{SafePath, Mode},
    record::Record,
    secret::Secret
};

use util::crypt::{CryptCtx, Header, Key};

use std::{
    process::ExitCode,
    fs::File
};

fn main() -> ExitCode {
    let result = Cmd::from_env()
        .map_err(Error::from)
        .and_then(Cmd::exec);

    match result {
        Ok(()) =>
            ExitCode::SUCCESS,

        Err(e) => {
            e.print_full();
            ExitCode::FAILURE
        }
    }
}

impl Cmd {
    fn exec(self) -> Result<()> {
        use Cmd::*;

        match Cmd::from_env()? {
            ShowUsage(usg) => println!("{usg}"),
            ShowVersion(ver) => println!("{ver}"),
            HandleFile(cmd, path) => cmd.exec(path)?,
        }

        Ok(())
    }
}

impl FileCmd {
    fn exec(self, path: SafePath) -> Result<()> {
        use FileCmd::*;

        if let Err(e) = recover::maybe_recover(&path) {
            return Err(Error::RecoveringBackup(e, path))
        }

        with_secured_mem(|| match self {
            Read(cmd)   => cmd.exec(path),
            Change(cmd) => cmd.exec(path),
            Create(cmd) => cmd.exec(path)
        })
    }
}

impl ReadCmd {
    fn exec(self, path: SafePath) -> Result<()> {
        use output::{PrintTarget, ClipTarget};
        use ReadCmd::*;

        let data = Secret::new({
            let (mut file, _) = open(Mode::Read, path)?;
            // TODO: maybe implement password retry if incorrect (also for other
            // cmds)
            let pw = Secret::new(input_pw::read("Password: ")?);
            let serial = Secret::new(decrypt(&mut file, &pw)?);

            if let Export = self {
                let ir = Secret::new(serial::ir_from(&serial)?);

                println!("{}", *ir);
                return Ok(());
            } else {
                serial::parse(&serial)?
            }
        });

        match self {
            Show(paths, mk) => PrintTarget::new(paths, mk)
                .print_values(&data),

            Clip(path, mk, time) => {
                // It doesn't matter if this is the parent or child process,
                // because it is about to exit without further effects.
                let _ = ClipTarget::new(path, mk, time)
                    .clip(&data)?;
            }

            List(opt_paths, mk) => match opt_paths {
                Some(paths) => PrintTarget::new(paths, mk)
                    .print_lists(&data),
                None => println!("{}", Record::display_list(&data))
            }

            Tree(opt_paths, mk) => match opt_paths {
                Some(paths) => PrintTarget::new(paths, mk)
                    .print_trees(&data),
                None => println!("{}", Record::display_tree(&data))
            }

            // Already handled.
            Export => unreachable!()
        }

        Ok(())
    }
}

impl ChangeCmd {
    fn exec(self, path: SafePath) -> Result<()> {
        use recover::Error::File as RecoverError;
        use ChangeCmd::*;
        use tui::{Tui, Status};
        use tui::Status::{Stopped, Clipped};

        let (mut file, path) = open(Mode::ReadWrite, path)?;
        let pw = Secret::new(input_pw::read("Password: ")?);
        let serial = Secret::new(decrypt(&mut file, &pw)?);

        if let Err(e) = path.make_backup() {
            return Err(Error::MakingBackup(e, path));
        }

        // TODO: use `try` blocks once available
        let result = move || -> Result<Status> {
            match self {
                Edit(config) => {
                    let data = Secret::new(serial::parse(&serial)?);
                    let mut tui = Tui::new(config);

                    drop(serial);   // Old serial data unneeded if changing.

                    // TODO: maybe launch this in separate proc/thread so we can
                    // catch ctrl-c and exit cleanly
                    tui.run(&data)?;

                    if tui.should_save_data() {
                        let new_serial = Secret::new(
                            serial::bytes_from(data)
                                .map_err(Error::SerialisingRecord)?
                        );

                        over_encrypt(&new_serial, file, &pw)?;
                    }

                    Ok(tui.status())
                }

                ChangePassword => {
                    drop(pw);   // Old password unneeded if we are changing it.

                    over_encrypt_with_input(
                        &serial,
                        file,
                        "New password: ",
                        "Confirm password: "
                    )?;

                    Ok(Stopped)
                }
            }
        }();

        match &result {
            // The main process will take care of the backup.
            Ok(Clipped) => (),

            Ok(_) => if let Err(e) = path.remove_backup() {
                Error::RemovingBackup(e, path).warn_full();
            }

            Err(_) => if let Err(e) = path.recover() {
                Error::RecoveringBackup(RecoverError(e), path).warn_full();
            }
        }

        result.map(|_| ())
    }
}

impl CreateCmd {
    fn exec(self, path: SafePath) -> Result<()> {
        use CreateCmd::*;

        let (file, path) = open(Mode::CreateWrite, path)?;

        // TODO: use `try` blocks once available
        let result = || -> Result<()> {
            let serial = match self {
                CreateEmpty(root_name) => {
                    Secret::new(serial::new_empty(root_name))
                }

                Import => {
                    let input = Secret::new(
                        user_io::read_stdin()
                            .map_err(Error::ReadingStdin)?
                    );

                    serial::validate(&input)
                        .map_err(Error::InputSerial)?;

                    input
                }
            };

            over_encrypt_with_input(
                serial.as_bytes(),
                file,
                "Password: ",
                "Confirm password: "
            )
        }();

        if result.is_err() {
            if let Err(e) = path.remove() {
                Error::RemovingFile(e, path).warn_full();
            }
        }

        result
    }
}

/// Runs `op` in a context where the process address space is secured by
/// [`proc::secure_mem`].
///
/// Returns the result of `op`, and prints a warning if [`proc::expose_mem`]
/// failed after the execution of `op`. Failure to reverse the memory
/// protections is not considered a fatal error.
fn with_secured_mem<O>(op: O) -> Result<()>
    where
        O: FnOnce() -> Result<()>
{
    proc::secure_mem()
        .map_err(Error::SecuringMemory)?;

    let result = op();

    if let Err(e) = proc::expose_mem() {
        Error::ExposingMemory(e).warn_full();
    }

    result
}

/// Opens the main path of `path` with `mode`.
///
/// Returns the opened file and the passed path unchanged.
fn open(mode: file::Mode, path: SafePath) -> Result<(File, SafePath)> {
    match path.open(mode) {
        Ok(f) => Ok((f, path)),
        Err(e) => Err(Error::OpeningFile(e, mode, path))
    }
}

/// reads header
fn decrypt(mut data: &mut File, pw: &str) -> Result<Vec<u8>> {
    let head = Header::read_from(&mut data)
        .map_err(Error::ReadingHeader)?;

    let key = Secret::new(
        Key::from_password(pw, &head)
            .map_err(input_pw::Error::GeneratingKey)?
    );

    let crypt_ctx = CryptCtx::new(&key, &head);

    Ok(crypt_ctx.decrypt(data)?)
}

/// generates new key and salt
/// and empties the file before writing
fn over_encrypt(data: &[u8], mut dest: File, pw: &str) -> Result<()> {
    let head = Header::generate();

    let key = Key::from_password(pw, &head)
        .map_err(input_pw::Error::GeneratingKey)?;

    let crypt_ctx = CryptCtx::new(&key, &head);

    file::clear(&mut dest)
        .map_err(Error::ClearingFile)?;

    head.write_to(&mut dest)
        .map_err(Error::WritingHeader)?;

    Ok(crypt_ctx.encrypt(data, &mut dest)?)
}

/// gets key from user input (password is asked twice for confirmation),
/// and generates key and salt
/// and empties the file before writing
fn over_encrypt_with_input(
    data: &[u8],
    mut dest: File,
    prompt_1: &str,
    prompt_2: &str
) -> Result<()> {
    let head = Header::generate();

    let key = Secret::new(input_pw::confirm_to_key(
        &head,
        prompt_1,
        prompt_2
    )?);

    file::clear(&mut dest)
        .map_err(Error::ClearingFile)?;

    head.write_to(&mut dest)
        .map_err(Error::WritingHeader)?;

    let crypt_ctx = CryptCtx::new(&key, &head);

    Ok(crypt_ctx.encrypt(data, &mut dest)?)
}
