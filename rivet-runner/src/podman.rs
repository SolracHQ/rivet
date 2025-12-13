//! Podman container management
//!
//! Handles container lifecycle for job execution:
//! - Checking podman availability
//! - Managing multiple containers per job
//! - Tracking container stack for nested container.run() calls
//! - Executing commands in containers
//! - Cleaning up all containers after job completion

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::process::Command;
use std::sync::Mutex;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Checks if podman is installed and available
pub fn check_podman_available() -> Result<()> {
    let output = Command::new("podman")
        .arg("--version")
        .output()
        .context("Failed to execute 'podman --version'. Is podman installed?")?;

    if !output.status.success() {
        anyhow::bail!("Podman is not working correctly");
    }

    let version = String::from_utf8_lossy(&output.stdout);
    info!("Podman is available: {}", version.trim());

    Ok(())
}

/// Container manager for a job
///
/// Manages multiple containers that can be created via container.run().
/// Tracks a stack of active containers, with the top being the current execution context.
pub struct ContainerManager {
    job_id: Uuid,
    workspace_path: String,

    /// Registry of all containers: image -> container_name
    containers: Mutex<HashMap<String, String>>,

    /// Stack of active container names (top = current context)
    stack: Mutex<Vec<String>>,
}

impl ContainerManager {
    /// Creates a new container manager
    ///
    /// # Arguments
    /// * `job_id` - The job ID
    /// * `workspace_path` - Path to workspace directory to mount in all containers
    pub fn new(job_id: Uuid, workspace_path: String) -> Self {
        Self {
            job_id,
            workspace_path,
            containers: Mutex::new(HashMap::new()),
            stack: Mutex::new(Vec::new()),
        }
    }

    /// Starts the default container and pushes it onto the stack
    ///
    /// # Arguments
    /// * `image` - Default container image (e.g., docker.io/alpine:latest)
    ///
    /// # Returns
    /// Container name
    pub fn start_default(&self, image: &str) -> Result<String> {
        info!(
            "Starting default container with image {} for job {}",
            image, self.job_id
        );

        let container_name = self.ensure_container_running(image)?;

        // Push to stack
        let mut stack = self.stack.lock().unwrap();
        stack.push(container_name.clone());

        info!(
            "Default container {} started and pushed to stack",
            container_name
        );
        Ok(container_name)
    }

    /// Ensures a container for the given image is running
    ///
    /// If container already exists, returns its name. Otherwise creates it.
    ///
    /// # Arguments
    /// * `image` - Container image to run
    ///
    /// # Returns
    /// Container name
    pub fn ensure_container_running(&self, image: &str) -> Result<String> {
        let mut containers = self.containers.lock().unwrap();

        // Check if container already exists for this image
        if let Some(container_name) = containers.get(image) {
            debug!(
                "Container {} already exists for image {}",
                container_name, image
            );
            return Ok(container_name.clone());
        }

        // Generate container name from image hash
        let container_name = self.generate_container_name(image);

        // Ensure workspace directory exists
        std::fs::create_dir_all(&self.workspace_path)
            .context("Failed to create workspace directory")?;

        info!("Creating container {} for image {}", container_name, image);

        // Start container with workspace mounted, sleeping indefinitely
        // podman run blocks until container is running, so no need to wait
        // Override entrypoint to /bin/sh to handle images with custom entrypoints (like alpine/git)
        let output = Command::new("podman")
            .arg("run")
            .arg("-d") // Detached
            .arg("--name")
            .arg(&container_name)
            .arg("--entrypoint")
            .arg("/bin/sh") // Override any image entrypoint
            .arg("-v")
            .arg(format!("{}:/workspace", self.workspace_path))
            .arg("-w")
            .arg("/workspace") // Set working directory
            .arg(image)
            .arg("-c")
            .arg("sleep infinity")
            .output()
            .context("Failed to execute podman run command")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Always log stdout/stderr as debug
        if !stdout.trim().is_empty() {
            debug!("podman run stdout: {}", stdout.trim());
        }
        if !stderr.trim().is_empty() {
            debug!("podman run stderr: {}", stderr.trim());
        }

        if !output.status.success() {
            let exit_code = output.status.code().unwrap_or(-1);

            let error_msg = format!(
                "Failed to start container for image {}: exit_code={}, stdout='{}', stderr='{}'",
                image,
                exit_code,
                stdout.trim(),
                stderr.trim()
            );

            error!("{}", error_msg);
            anyhow::bail!("{}", error_msg);
        }

        let container_id = stdout.trim().to_string();
        info!(
            "Container {} started successfully with ID: {}",
            container_name, container_id
        );

        // Register container
        containers.insert(image.to_string(), container_name.clone());

        Ok(container_name)
    }

    /// Pushes a container onto the stack
    ///
    /// Used by container.run() to switch execution context.
    /// The container for the given image will be created if it doesn't exist.
    ///
    /// # Arguments
    /// * `image` - Container image to push
    ///
    /// # Returns
    /// Container name
    pub fn push_container(&self, image: &str) -> Result<String> {
        let container_name = self.ensure_container_running(image)?;

        let mut stack = self.stack.lock().unwrap();
        stack.push(container_name.clone());

        debug!(
            "Pushed container {} onto stack (depth: {})",
            container_name,
            stack.len()
        );
        Ok(container_name)
    }

    /// Pops a container from the stack
    ///
    /// Used when container.run() block completes.
    ///
    /// # Returns
    /// The popped container name, or None if stack is empty
    pub fn pop_container(&self) -> Option<String> {
        let mut stack = self.stack.lock().unwrap();
        let popped = stack.pop();

        if let Some(ref name) = popped {
            debug!(
                "Popped container {} from stack (depth: {})",
                name,
                stack.len()
            );
        }

        popped
    }

    /// Gets the current container name from the top of the stack
    ///
    /// # Returns
    /// Current container name, or None if stack is empty
    pub fn current_container(&self) -> Option<String> {
        let stack = self.stack.lock().unwrap();
        stack.last().cloned()
    }

    /// Executes a command in the current container
    ///
    /// # Arguments
    /// * `cmd` - Command to execute
    /// * `args` - Arguments for the command
    /// * `cwd` - Working directory (relative to /workspace, None = /workspace)
    ///
    /// # Returns
    /// (stdout, stderr, exit_code)
    pub fn exec(
        &self,
        cmd: &str,
        args: &[String],
        cwd: Option<&str>,
    ) -> Result<(String, String, i32)> {
        let container_name = self
            .current_container()
            .ok_or_else(|| anyhow::anyhow!("No active container in stack"))?;

        debug!(
            "Executing in container {}: {} {:?}",
            container_name, cmd, args
        );

        let working_dir = match cwd {
            Some(dir) => {
                if dir.starts_with('/') {
                    dir.to_string()
                } else {
                    format!("/workspace/{}", dir)
                }
            }
            None => "/workspace".to_string(),
        };

        let mut command = Command::new("podman");
        command
            .arg("exec")
            .arg("-w")
            .arg(&working_dir)
            .arg(&container_name)
            .arg(cmd);

        for arg in args {
            command.arg(arg);
        }

        let output = command
            .output()
            .context("Failed to execute podman exec command")?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(1);

        if !output.status.success() {
            debug!(
                "Command failed in container {}: cmd={} exit_code={} stdout='{}' stderr='{}'",
                container_name,
                cmd,
                exit_code,
                stdout.trim(),
                stderr.trim()
            );
        } else {
            debug!(
                "Command completed successfully: exit_code={}, stdout_len={}, stderr_len={}",
                exit_code,
                stdout.len(),
                stderr.len()
            );
        }

        Ok((stdout, stderr, exit_code))
    }

    /// Stops and removes all containers created by this manager
    pub fn cleanup(&self) -> Result<()> {
        let containers = self.containers.lock().unwrap();

        info!(
            "Cleaning up {} container(s) for job {}",
            containers.len(),
            self.job_id
        );

        for (image, container_name) in containers.iter() {
            debug!("Stopping container {} (image: {})", container_name, image);

            // Stop container (ignore errors if already stopped)
            let _ = Command::new("podman")
                .arg("stop")
                .arg(container_name)
                .output();

            // Remove container
            let rm_output = Command::new("podman")
                .arg("rm")
                .arg("-f") // Force remove
                .arg(container_name)
                .output();

            match rm_output {
                Ok(output) if output.status.success() => {
                    debug!("Container {} removed", container_name);
                }
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    warn!("Failed to remove container {}: {}", container_name, stderr);
                }
                Err(e) => {
                    warn!("Failed to remove container {}: {}", container_name, e);
                }
            }
        }

        info!("Cleanup complete for job {}", self.job_id);
        Ok(())
    }

    /// Generates a unique container name for a job and image
    ///
    /// Uses a simple hash of the image name to ensure consistent naming
    fn generate_container_name(&self, image: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        image.hash(&mut hasher);
        let hash = hasher.finish();

        format!("rivet-{}-{:x}", self.job_id, hash)
    }
}

impl Drop for ContainerManager {
    fn drop(&mut self) {
        if let Err(e) = self.cleanup() {
            warn!("Failed to cleanup containers on drop: {}", e);
        }
    }
}
