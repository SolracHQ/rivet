return {
    name = "Example Pipeline",
    description = "A simple pipeline that demonstrates logging",

    inputs = {
        message = {
            type = "string",
            description = "A message to log",
            default = "Hello from Rivet!"
        }
    },

    stages = {
        {
            name = "checkout",
            script = function()
                log.info("Starting checkout stage...")
                log.debug("This is a debug message")
                log.info("Checkout completed successfully")
            end
        },
        {
            name = "test",
            script = function()
                log.info("Starting test stage...")
                local message = env.get("message", "default message")
                log.info("Message from environment: " .. message)
                log.warning("This is a warning message")
                log.info("Tests completed successfully")
            end
        },
        {
            name = "deploy",
            script = function()
                log.info("Starting deploy stage...")
                log.info("Deploying application...")
                log.info("Deployment completed successfully")
            end
        }
    }
}
