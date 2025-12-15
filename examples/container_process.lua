return pipeline.define({
    name = "Container & Process Pipeline",
    description = "Demonstrates container.with(), process.run(), input, and log modules",

    inputs = {
        git_repo = {
            type = "string",
            description = "Git repository URL to clone (leave empty to skip)",
            default = ""
        },
        git_branch = {
            type = "string",
            description = "Git branch to checkout",
            default = "master"
        },
        build_target = {
            type = "string",
            description = "Make target to build",
            default = "help"
        }
    },

    stages = {
        {
            name = "default_container_test",
            script = function()
                log.info("Testing default Alpine container...")

                -- These run in the default alpine container
                local result = process.run({
                    cmd = "uname",
                    args = { "-a" },
                    capture_stdout = true
                })
                log.info("System info: " .. result.stdout)

                -- Check Alpine version
                local alpine_result = process.run({
                    cmd = "cat",
                    args = { "/etc/alpine-release" },
                    capture_stdout = true
                })
                log.info("Alpine version: " .. alpine_result.stdout)

                log.info("Default container test completed")
            end
        },

        {
            name = "git_operations",
            script = function()
                log.info("Starting git operations...")

                local repo = input.get("git_repo", "")
                local branch = input.get("git_branch", "master")

                if repo == "" then
                    log.warning("No git_repo provided, skipping clone")
                    return
                end

                -- Use alpine/git container for git operations
                container.with("docker.io/alpine/git:latest", function()
                    log.info("Cloning repository: " .. repo)
                    log.info("Branch: " .. branch)

                    -- Clone with depth 1 for speed
                    local clone_result = process.run({
                        cmd = "git",
                        args = { "clone", "--branch", branch, "--depth", "1", repo, "repo" },
                        stderr_level = "info"
                    })

                    if clone_result.exit_code ~= 0 then
                        log.error("Git clone failed with exit code: " .. clone_result.exit_code)
                        error("Failed to clone repository")
                    end

                    -- Show git log
                    local log_result = process.run({
                        cmd = "git",
                        args = { "-C", "repo", "log", "-1", "--oneline" },
                        capture_stdout = true
                    })
                    log.info("Latest commit: " .. log_result.stdout)

                    -- Count files
                    local count_result = process.run({
                        cmd = "sh",
                        args = { "-c", "find repo -type f | wc -l" },
                        capture_stdout = true
                    })
                    log.info("Files in repository: " .. count_result.stdout)
                end)

                log.info("Git operations completed")
            end
        },

        {
            name = "python_environment",
            script = function()
                log.info("Testing Python environment...")

                -- Use Python container for Python tasks
                container.with("docker.io/python:3.11-alpine", function()
                    -- Check Python version
                    local version_result = process.run({
                        cmd = "python",
                        args = { "--version" },
                        capture_stdout = true
                    })
                    log.info("Python version: " .. version_result.stdout)

                    -- Run simple Python script
                    local python_result = process.run({
                        cmd = "python",
                        args = { "-c", "import sys; print(f'Python {sys.version}'); print(f'Platform: {sys.platform}')" },
                        capture_stdout = true,
                        stdout_level = "debug"
                    })

                    if python_result.exit_code == 0 then
                        log.info("Python script executed successfully")
                    end

                    -- Test pip
                    process.run({
                        cmd = "pip",
                        args = { "--version" },
                        stdout_level = "debug"
                    })
                end)

                log.info("Python environment test completed")
            end
        },

        {
            name = "nested_containers",
            script = function()
                log.info("Testing nested container execution...")

                -- Outer container: Alpine
                container.with("docker.io/alpine:latest", function()
                    log.info("In Alpine container")

                    -- Install curl in alpine
                    process.run({
                        cmd = "apk",
                        args = { "add", "--no-cache", "curl" },
                        stdout_level = "debug"
                    })

                    -- Inner container: Python (overrides Alpine for this block)
                    container.with("docker.io/python:3.11-alpine", function()
                        log.info("In Python container (nested)")

                        local result = process.run({
                            cmd = "python",
                            args = { "--version" },
                            capture_stdout = true
                        })
                        log.info("Nested container Python: " .. result.stdout)
                    end)

                    -- Back to Alpine
                    log.info("Back in Alpine container")
                    local curl_result = process.run({
                        cmd = "curl",
                        args = { "--version" },
                        capture_stdout = true
                    })
                    log.info("Curl version: " .. curl_result.stdout:match("[^\n]+")) -- first line only
                end)

                log.info("Nested container test completed")
            end
        },

        {
            name = "working_directory_test",
            script = function()
                log.info("Testing working directory...")

                -- Create directory structure
                process.run({
                    cmd = "mkdir",
                    args = { "-p", "test/subdir" }
                })

                -- Create file in subdir
                process.run({
                    cmd = "sh",
                    args = { "-c", "echo 'Hello from subdir' > test/subdir/file.txt" }
                })

                -- Read file with cwd
                local result = process.run({
                    cmd = "cat",
                    args = { "file.txt" },
                    cwd = "test/subdir",
                    capture_stdout = true
                })
                log.info("File content: " .. result.stdout)

                -- List files
                process.run({
                    cmd = "find",
                    args = { "test", "-type", "f" },
                    stdout_level = "debug"
                })

                log.info("Working directory test completed")
            end
        },

        {
            name = "error_handling",
            script = function()
                log.info("Testing error handling...")

                -- Command that fails
                local fail_result = process.run({
                    cmd = "ls",
                    args = { "/nonexistent/path" },
                    capture_stderr = true
                })

                if fail_result.exit_code ~= 0 then
                    log.warning("Expected failure occurred with exit code: " .. fail_result.exit_code)
                    log.debug("Error output: " .. (fail_result.stderr or ""))
                else
                    log.error("Expected command to fail but it succeeded!")
                end

                -- Test container with error handling
                local success, err = pcall(function()
                    container.with("docker.io/invalid/nonexistent:tag", function()
                        log.info("This should not execute")
                    end)
                end)

                if not success then
                    log.warning("Expected container error caught: " .. tostring(err))
                end

                log.info("Error handling test completed")
            end
        },

        {
            name = "build_example",
            script = function()
                log.info("Build example with make (if repo was cloned)...")

                -- Check if repo directory exists
                local check_result = process.run({
                    cmd = "test",
                    args = { "-d", "repo" }
                })

                if check_result.exit_code ~= 0 then
                    log.warning("Repo directory not found, skipping build")
                    return
                end

                local target = input.get("build_target", "help")
                log.info("Building target: " .. target)

                -- Run make in the cloned repo
                local make_result = process.run({
                    cmd = "make",
                    args = { target },
                    cwd = "repo",
                    stdout_level = "debug",
                    stderr_level = "warning"
                })

                if make_result.exit_code == 0 then
                    log.info("Build completed successfully")
                else
                    log.warning("Build exited with code: " .. make_result.exit_code)
                end

                log.info("Build example completed")
            end
        },

        {
            name = "summary",
            script = function()
                log.info("=== Pipeline Summary ===")
                log.info("All stages completed successfully!")
                log.info("Demonstrated features:")
                log.info("  - Default Alpine container")
                log.info("  - Git operations in alpine/git container")
                log.info("  - Python execution in python:3.11 container")
                log.info("  - Nested container contexts")
                log.info("  - Working directory changes")
                log.info("  - Error handling")
                log.info("  - Process output capture")
                log.info("  - Input parameter usage")
                log.info("========================")
            end
        }
    }
})
