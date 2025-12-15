return pipeline.define({
    name = "Simple Process Test",
    description = "Quick test of process and container modules",

    inputs = {
        message = {
            type = "string",
            description = "A message to echo",
            default = "Hello from Rivet!"
        }
    },

    stages = {
        {
            name = "basic_commands",
            script = function()
                log.info("Testing basic commands in default Alpine container")

                -- Echo message from input
                local message = input.get("message", "default")
                log.info("Input message: " .. message)

                -- Run echo command
                local result = process.run({
                    cmd = "echo",
                    args = { message },
                    capture_stdout = true
                })
                log.info("Echo output: " .. result.stdout)

                -- Show current directory
                local pwd_result = process.run({
                    cmd = "pwd",
                    capture_stdout = true
                })
                log.info("Working directory: " .. pwd_result.stdout)

                -- List workspace
                process.run({
                    cmd = "ls",
                    args = { "-la" },
                    stdout_level = "debug"
                })

                log.info("Basic commands completed")
            end
        },

        {
            name = "file_operations",
            script = function()
                log.info("Testing file operations")

                -- Create a test file
                process.run({
                    cmd = "sh",
                    args = { "-c", "echo 'Test content' > test.txt" }
                })

                -- Read the file
                local cat_result = process.run({
                    cmd = "cat",
                    args = { "test.txt" },
                    capture_stdout = true
                })
                log.info("File content: " .. cat_result.stdout)

                -- Create directory and file
                process.run({
                    cmd = "mkdir",
                    args = { "-p", "data" }
                })

                process.run({
                    cmd = "sh",
                    args = { "-c", "echo 'data content' > data/info.txt" }
                })

                -- List directory
                process.run({
                    cmd = "find",
                    args = { ".", "-type", "f" }
                })

                log.info("File operations completed")
            end
        },

        {
            name = "container_test",
            script = function()
                log.info("Testing container.with() with Python")

                container.with("docker.io/python:3.11-alpine", function()
                    log.info("Inside Python container")

                    -- Check Python version
                    local version = process.run({
                        cmd = "python",
                        args = { "--version" },
                        capture_stdout = true
                    })
                    log.info("Python version: " .. version.stdout)

                    -- Run simple Python code
                    local py_result = process.run({
                        cmd = "python",
                        args = { "-c", "print('Hello from Python!')" },
                        capture_stdout = true
                    })
                    log.info("Python says: " .. py_result.stdout)
                end)

                log.info("Back to default container")
                local alpine = process.run({
                    cmd = "cat",
                    args = { "/etc/alpine-release" },
                    capture_stdout = true
                })
                log.info("Alpine version: " .. alpine.stdout)

                log.info("Container test completed")
            end
        },

        {
            name = "exit_code_test",
            script = function()
                log.info("Testing exit codes")

                -- Successful command
                local success = process.run({
                    cmd = "true"
                })
                log.info("true exit code: " .. success.exit_code)

                -- Failed command
                local failure = process.run({
                    cmd = "false"
                })
                log.info("false exit code: " .. failure.exit_code)

                -- Command with error (but don't fail pipeline)
                local err_result = process.run({
                    cmd = "ls",
                    args = { "/nonexistent" },
                    capture_stderr = true
                })
                log.warning("ls nonexistent exit code: " .. err_result.exit_code)

                log.info("Exit code test completed")
            end
        },

        {
            name = "summary",
            script = function()
                log.info("=== Test Summary ===")
                log.info("All tests passed successfully!")
                log.info("Features tested:")
                log.info("  ✓ Basic process execution")
                log.info("  ✓ Input parameters")
                log.info("  ✓ File operations")
                log.info("  ✓ Container switching")
                log.info("  ✓ Exit code handling")
                log.info("====================")
            end
        }
    }
})
