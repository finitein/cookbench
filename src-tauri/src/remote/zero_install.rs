//! Zero-install discovery. It only asks an existing SSH host for session-file
//! names and never deploys the optional bridge.

use cookbench_core::remote::{PollInterval, RemoteHost, SessionRoot};

use super::{
    reconnect::ReconnectState,
    ssh::{session_paths, SshError, SshInvocation, SshRunner},
};

pub struct ZeroInstallSshSource<R> {
    host: RemoteHost,
    runner: R,
    reconnect: ReconnectState,
}

impl<R: SshRunner> ZeroInstallSshSource<R> {
    pub fn new(host: RemoteHost, runner: R) -> Self {
        Self {
            host,
            runner,
            reconnect: ReconnectState::default(),
        }
    }

    pub fn host(&self) -> &RemoteHost {
        &self.host
    }

    pub fn poll_interval(&self) -> PollInterval {
        self.reconnect.next_interval()
    }

    /// Returns `true` only when a previously disconnected host has recovered.
    pub fn discover(&mut self) -> Result<(Vec<String>, bool), SshError> {
        let roots: Vec<SessionRoot> = self.host.session_roots().to_vec();
        let result = roots.into_iter().try_fold(Vec::new(), |mut paths, root| {
            let output = self
                .runner
                .run(&SshInvocation::discover(&self.host, &root))?;
            paths.extend(session_paths(&output, &root)?);
            Ok::<_, SshError>(paths)
        });

        match result {
            Ok(paths) => {
                let restored = self.reconnect.record_success(!paths.is_empty());
                Ok((paths, restored))
            }
            Err(error) => {
                self.reconnect.record_failure(&error);
                Err(error)
            }
        }
    }
}
