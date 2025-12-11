pipeline = {
    name = "Example Pipeline",
    stages = {
        {
            name = "checkout",
            script = function()
                log.info("Cloning repository...")
            end
        },
        {
            name = "test",
            script = function()
                log.info("Running tests...")
            end
        }
    }
}
