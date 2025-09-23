use std::collections::HashMap;
use std::process::Child;
use std::sync::{Arc, Mutex};
use tokio::signal;

#[cfg(unix)]
extern crate libc;

/// Signal that should be forwarded to child processes
#[derive(Debug, Clone, Copy)]
pub enum Signal {
    Interrupt, // SIGINT (Ctrl+C)
    Terminate, // SIGTERM
}

/// Handle to a child process that can receive signals
pub struct ProcessHandle {
    pub pid: u32,
    pub child: Arc<Mutex<Option<Child>>>,
}

impl ProcessHandle {
    pub fn new(child: Child) -> Self {
        let pid = child.id();
        Self {
            pid,
            child: Arc::new(Mutex::new(Some(child))),
        }
    }

    /// Send a signal to this process
    pub fn send_signal(&self, signal: Signal) -> anyhow::Result<()> {
        let mut child_guard = self.child.lock().unwrap();
        if let Some(child) = child_guard.as_mut() {
            match signal {
                Signal::Interrupt => {
                    // Try to send SIGINT to the process group
                    #[cfg(unix)]
                    {
                        unsafe {
                            // Send SIGINT to the process group
                            libc::kill(-(self.pid as i32), libc::SIGINT);
                        }
                    }
                    #[cfg(not(unix))]
                    {
                        // On non-Unix systems, just kill the process
                        let _ = child.kill();
                    }
                }
                Signal::Terminate => {
                    // Try to send SIGTERM to the process group first, then kill if needed
                    #[cfg(unix)]
                    {
                        unsafe {
                            libc::kill(-(self.pid as i32), libc::SIGTERM);
                        }
                        // Give it a moment to terminate gracefully
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        // Check if it's still running, if so, kill it
                        if child.try_wait().unwrap_or(None).is_none() {
                            let _ = child.kill();
                        }
                    }
                    #[cfg(not(unix))]
                    {
                        let _ = child.kill();
                    }
                }
            }
        }
        Ok(())
    }

    /// Wait for the process to complete and take ownership back
    pub fn wait(self) -> anyhow::Result<std::process::ExitStatus> {
        let mut child_guard = self.child.lock().unwrap();
        if let Some(mut child) = child_guard.take() {
            Ok(child.wait()?)
        } else {
            anyhow::bail!("Process has already been waited on or killed")
        }
    }

    /// Check if process is still running
    pub fn try_wait(&self) -> anyhow::Result<Option<std::process::ExitStatus>> {
        let mut child_guard = self.child.lock().unwrap();
        if let Some(child) = child_guard.as_mut() {
            Ok(child.try_wait()?)
        } else {
            // Process already finished - create a success exit status
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                Ok(Some(std::process::ExitStatus::from_raw(0)))
            }
            #[cfg(not(unix))]
            {
                // On non-Unix systems, we'll assume success
                // This is a bit of a hack, but ExitStatus creation is platform-specific
                Ok(None)
            }
        }
    }
}

/// Global signal manager that tracks all running processes
pub struct SignalManager {
    processes: Arc<Mutex<HashMap<u32, ProcessHandle>>>,
    next_id: Arc<Mutex<u32>>,
}

impl SignalManager {
    pub fn new() -> Self {
        panic!();
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(0)),
        }
    }

    /// Register a new process to be managed
    pub fn register_process(&self, child: Child) -> anyhow::Result<ProcessHandle> {
        // Set the process group for the child
        #[cfg(unix)]
        {
            // The process group should have been set when spawning the command
            // This is just a safeguard
        }

        let handle = ProcessHandle::new(child);
        let id = {
            let mut next_id = self.next_id.lock().unwrap();
            *next_id += 1;
            *next_id
        };

        {
            let mut processes = self.processes.lock().unwrap();
            processes.insert(id, ProcessHandle {
                pid: handle.pid,
                child: handle.child.clone(),
            });
        }

        Ok(handle)
    }

    /// Send a signal to all registered processes
    pub fn signal_all(&self, signal: Signal) {
        let processes = self.processes.lock().unwrap();
        for (_, process) in processes.iter() {
            if let Err(e) = process.send_signal(signal) {
                eprintln!("Failed to send signal to process {}: {}", process.pid, e);
            }
        }
    }

    /// Remove a process from management (called when process completes)
    pub fn unregister_process(&self, pid: u32) {
        let mut processes = self.processes.lock().unwrap();
        processes.retain(|_, p| p.pid != pid);
    }

    /// Clean up finished processes
    pub fn cleanup_finished(&self) {
        let mut processes = self.processes.lock().unwrap();
        processes.retain(|_, p| {
            match p.try_wait() {
                Ok(Some(_)) => false, // Process finished, remove it
                Ok(None) => true,     // Process still running, keep it
                Err(_) => false,      // Error checking, assume finished
            }
        });
    }
}

static GLOBAL_SIGNAL_MANAGER: std::sync::OnceLock<SignalManager> = std::sync::OnceLock::new();

/// Get the global signal manager instance
pub fn get_signal_manager() -> &'static SignalManager {
    GLOBAL_SIGNAL_MANAGER.get_or_init(|| SignalManager::new())
}

/// Initialize signal handlers for the application
pub async fn init_signal_handlers() -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        let manager = get_signal_manager();
        
        // Handle SIGINT (Ctrl+C)
        let manager_int = manager;
        tokio::spawn(async move {
            let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())
                .expect("Failed to create SIGINT handler");
            
            while sigint.recv().await.is_some() {
                eprintln!("\nReceived SIGINT, forwarding to child processes...");
                manager_int.signal_all(Signal::Interrupt);
                // Give child processes time to handle the signal
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                std::process::exit(130); // Exit with SIGINT exit code
            }
        });

        // Handle SIGTERM
        let manager_term = manager;
        tokio::spawn(async move {
            let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("Failed to create SIGTERM handler");
            
            while sigterm.recv().await.is_some() {
                eprintln!("\nReceived SIGTERM, forwarding to child processes...");
                manager_term.signal_all(Signal::Terminate);
                // Give child processes time to handle the signal
                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                std::process::exit(143); // Exit with SIGTERM exit code
            }
        });
    }

    #[cfg(not(unix))]
    {
        // On non-Unix systems, we can't handle signals the same way
        // Just set up a basic Ctrl+C handler
        tokio::spawn(async {
            tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl_c");
            eprintln!("\nReceived Ctrl+C, exiting...");
            std::process::exit(130);
        });
    }

    Ok(())
}
