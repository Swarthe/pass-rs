use nix::{
    sys::{mman, resource},
    unistd
};

use nix::sys::{
    mman::MlockAllFlags,
    resource::Resource
};

use nix::unistd::{ForkResult, Pid};

#[derive(Clone, Copy, PartialEq, Eq)]
// Our version of `ForkResult` has a `must_use` attribute, which encourages the
// user to handle it. This is vital, as the child process is likely intended to
// follow a different path of execution.
#[must_use = "the currently executing process could be the child or parent"]
pub enum Process {
    Child,
    Parent { child: Pid }
}

pub type Error = nix::Error;

pub type Result<T> = std::result::Result<T, Error>;

impl From<ForkResult> for Process {
    fn from(f: ForkResult) -> Self {
        use ForkResult::{Child, Parent};

        match f {
            Child => Self::Child,
            Parent { child } => Self::Parent { child }
        }
    }
}

/// XXX: refer to manpages

/// Locks all current and future mapped memory pages, preventing them from being
/// swapped to disk, and disables process core dumps.
///
/// This can be used to avoid leaking passwords and other sensitive data to the
/// disk, thus potentially exposing it to external actors.
pub fn secure_mem() -> Result<()> {
    mman::mlockall(MlockAllFlags::all())?;
    disable_dumps()?;

    Ok(())
}

/// Reverses the effects of [`secure_mem`].
pub fn expose_mem() -> Result<()> {
    mman::munlockall()?;
    // Linux does not allow core dumps to be re-enabled after having been
    // disabled, even as root.
    // <https://github.com/sudo-project/sudo/blob/main/src/limits.c#L252>
    #[cfg(not(target_os = "linux"))]
    enable_dumps()?;

    Ok(())
}

/// # Safety
///
/// This function is completely safe if called from a single-threaded process.
/// However, the newly created process is not an exact duplicate of the
/// original. For example, it does not inherit its parent's memory locks (such
/// as those applied by [`secure_mem`]). Further differences are available at
/// the [`fork(2)`] man page.
///
/// If called from a multi-threaded program, undefined behaviour is possible
/// under circumstances outlined in the documentation for [`unistd::fork`]. In
/// particular, only async safe functions may be called from the child process.
///
/// [`fork(2)`]: https://man7.org/linux/man-pages/man2/fork.2.html
pub unsafe fn fork() -> Result<Process> {
    Ok(unistd::fork()?.into())
}

fn disable_dumps() -> Result<()> {
    resource::setrlimit(Resource::RLIMIT_CORE, 0, 0)
}

/// XXX: doesnt work on linux, even as root
///       <https://github.com/sudo-project/sudo/blob/main/src/limits.c#L252>
#[allow(unused)]    // Unused on Linux.
fn enable_dumps() -> Result<()> {
    use nix::libc::RLIM_INFINITY;

    resource::setrlimit(
        Resource::RLIMIT_CORE,
        RLIM_INFINITY,
        RLIM_INFINITY
    )
}
