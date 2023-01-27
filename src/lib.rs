mod customsource;

gst::plugin_define!(
    customsource,
    env!("CARGO_PKG_DESCRIPTION"),
    plugin_init,
    concat!(env!("CARGO_PKG_VERSION"), "-", env!("COMMIT_ID")),
    env!("CARGO_PKG_LICENSE"),
    env!("CARGO_PKG_NAME"),
    env!("CARGO_PKG_NAME"),
    env!("CARGO_PKG_REPOSITORY"),
    env!("BUILD_REL_DATE")
);

fn plugin_init(plugin: &gst::Plugin) -> Result<(), gst::glib::BoolError> {
    customsource::register(plugin)?;
    Ok(())
}