use gst::glib;
use gst::prelude::*;

glib::wrapper! {
    pub struct CustomSource(ObjectSubclass<imp::CustomSource>) @extends gst::Bin, gst::Element, gst::Object, @implements gst::URIHandler;
}

pub fn register(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    gst::Element::register(
        Some(plugin),
        "customsource",
        gst::Rank::Primary,
        CustomSource::static_type(),
    )
}

mod imp {
    use gst::glib;
    use gst::prelude::*;
    use gst::subclass::prelude::*;
    use url::Url;

    use std::str::FromStr;
    use std::{sync::Mutex};

    use once_cell::sync::Lazy;

    static CAT: Lazy<gst::DebugCategory> = Lazy::new(|| {
        gst::DebugCategory::new(
            "CustomSource",
            gst::DebugColorFlags::empty(),
            Some("Custom Source Bin"),
        )
    });

    struct State {
        filesrc: gst::Element,
    }

    pub struct CustomSource {
        srcpad: gst::GhostPad,
        state: Mutex<Option<State>>,
    }

    impl CustomSource {
        fn build_state() -> Result<State, glib::Error>{
            let filesrc = gst::ElementFactory::make("filesrc")
                .name("filesrc")
                .build()
                .unwrap();

            Ok(State { filesrc })
        }

        fn range(
            &self,
            pad: &gst::GhostPad,
            offset: u64,
            buffer: Option<&mut gst::BufferRef>,
            size: u32,
        ) -> Result<gst::PadGetRangeSuccess, gst::FlowError> {
            let target_src_pad = pad.target().unwrap();

            gst::debug!(CAT, obj: pad, "range: {pad:?}");

            let ret = match buffer {
                    Some(buffer) => {
                        match target_src_pad.range_fill(offset, buffer, size) {
                            Ok(..) => Ok(gst::PadGetRangeSuccess::FilledBuffer),
                            Err(err) => {
                                gst::error!(CAT, obj: pad, "Error: {err:?}");
                                Err(err)
                            },
                        }
                    },
                    None => {
                        match target_src_pad.range(offset, size) {
                            Ok(buffer) => Ok(gst::PadGetRangeSuccess::NewBuffer(buffer)),
                            Err(err) => {
                                gst::error!(CAT, obj: pad, "Error: {err:?}");
                                Err(err)
                            },
                        }
                    }
                };

            gst::debug!(CAT, obj: pad, "end range: {ret:?}");

            ret
        }

        fn pad_activate(&self, pad: &gst::GhostPad) -> Result<(), gst::LoggableError> {
            gst::debug!(CAT, obj: pad, "activate {pad:?}");

            pad.activate_mode(gst::PadMode::Pull, true)?;
            Ok(())
        }

        fn src_activatemode(
            &self,
            pad: &gst::GhostPad,
            mode: gst::PadMode,
            active: bool,
        ) -> Result<(), gst::LoggableError> {
            gst::debug!(CAT, obj: pad, "activatemode: {mode:?} ({active:?})");

            match mode {
                gst::PadMode::Pull => {
                    pad
                        .target()
                        .unwrap()
                        .activate_mode(mode, active)
                        .map_err(gst::LoggableError::from)?;

                    Ok(())
                }
                gst::PadMode::Push => Err(gst::loggable_error!(CAT, "Push mode not supported")),
                _ => Err(gst::loggable_error!(
                    CAT,
                    "Failed to activate the pad in Unknown mode, {:?}",
                    mode
                )),
            }
        }

        fn src_event(&self, pad: &gst::GhostPad, event: gst::Event) -> bool {
            gst::log!(CAT, obj: pad, "Handling event on srcpad {:?}", event.view());
            gst::Pad::event_default(pad, Some(&*self.obj()), event)
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CustomSource {
        const NAME: &'static str = "CustomSource";
        type Type = super::CustomSource;
        type ParentType = gst::Bin;
        type Interfaces = (gst::URIHandler,);

        fn with_class(klass: &Self::Class) -> Self {
            let templ = klass.pad_template("src").unwrap();

            let srcpad = gst::GhostPad::builder_with_template(&templ, Some("src"))
                .getrange_function(|pad, parent, offset, buffer, size| {
                    CustomSource::catch_panic_pad_function(
                        parent,
                        || Err(gst::FlowError::Error),
                        |source| source.range(pad, offset, buffer, size),
                    )
                })
                .activate_function(|pad, parent| {
                    CustomSource::catch_panic_pad_function(
                        parent,
                        || {
                            Err(gst::loggable_error!(
                                CAT,
                                "Panic activating srcpad with mode"
                            ))
                        },
                        |source| source.pad_activate(pad),
                    )
                })
                .activatemode_function(|pad, parent, mode, active| {
                    CustomSource::catch_panic_pad_function(
                        parent,
                        || { 
                            Err(gst::loggable_error!(
                                CAT,
                                "Panic activating srcpad with mode"
                            ))
                        },
                        |customsource| customsource.src_activatemode(pad, mode, active),
                    )
                })
                .event_function(|pad, parent, event| {
                    CustomSource::catch_panic_pad_function(
                        parent,
                        || false,
                        |source| source.src_event(pad, event),
                    )
                })
                .build();

            Self {
                srcpad,
                state: Mutex::new(None),
            }
        } 
    }

    impl ObjectImpl for CustomSource {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| -> Vec<glib::ParamSpec> {
                vec![
                    glib::ParamSpecString::builder("location")
                        .nick("File location")
                        .blurb("Location of the file to read")
                        .build(),
                ]
            });

            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            let state = self.state.lock().unwrap();

            gst::debug!(CAT, "Setting property {:?}: {value:?}", pspec.name());
            if let Some(state) = &*state {
                match pspec.name() {
                    "location" => {
                        let location = value.get::<String>().unwrap();

                        gst::debug!(CAT, imp: self, "Setting filesrc location: {location:?}");
                        state.filesrc.set_property("location", location.as_str());
                    },
                    _ => (),
                }
            }
            else {
                gst::error!(CAT, "Cannot set properties before internal state has been built");
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            gst::debug!(CAT, "Getting property: {:?}", pspec.name());

            let state = self.state.lock().unwrap();

            if let Some(state) = &*state {
                match pspec.name() {
                    "location" => {
                        state.filesrc.property::<Option<String>>("location").to_value()
                    },
                    _ => unimplemented!(),
                }
            }
            else {
                gst::element_error!(
                    self.obj(),
                    gst::LibraryError::Failed,
                    ["Cannot get property before state has been built"]);

                unimplemented!()
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            *self.state.lock().unwrap() = match CustomSource::build_state() {
                Ok(state) => {
                    let obj = self.obj();

                    obj.add_many(&[
                        &state.filesrc,
                    ]).expect("Could not add elements to bin");

                    self.srcpad.set_target(Some(&state.filesrc.static_pad("src").unwrap())).expect("Set ghostpad target failed");

                    if let Err(err) = obj.add_pad(&self.srcpad) {
                        gst::error!(CAT, imp: self, "Error adding pad to element: {err:?}");
                        None
                    }
                    else {
                        Some(state) 
                    }
                },
                Err(err) => {
                    gst::error!(CAT, imp: self, "Error building state: {err:?}");
                    None
                },
            }

        }
    }

    impl GstObjectImpl for CustomSource {}

    impl ElementImpl for CustomSource {
        fn metadata() -> Option<&'static gst::subclass::ElementMetadata> {
            static ELEMENT_METADATA: Lazy<gst::subclass::ElementMetadata> = Lazy::new(|| {
                gst::subclass::ElementMetadata::new(
                    "CustomSource Bin",
                    "Generic",
                    "Custom source",
                    "Rodrigo Santos <rsantos@sequence.film>",
                )
            });

            Some(&*ELEMENT_METADATA)
        }

        fn pad_templates() -> &'static [gst::PadTemplate] {
            static PAD_TEMPLATES: Lazy<Vec<gst::PadTemplate>> = Lazy::new(|| {
                let src_pad_template = gst::PadTemplate::new(
                    "src",
                    gst::PadDirection::Src,
                    gst::PadPresence::Always,
                    &gst::Caps::new_any(),
                )
                .unwrap();

                vec![src_pad_template]
            });

            PAD_TEMPLATES.as_ref()
        }

        fn change_state(
            &self,
            transition: gst::StateChange,
        ) -> Result<gst::StateChangeSuccess, gst::StateChangeError> {

            gst::debug!(CAT, imp: self, "{transition:?}");

            // Call the parent class' implementation of ::change_state()
            let ret = self.parent_change_state(transition);

            gst::debug!(CAT, imp: self, "{ret:?}");
            ret
        }
    }

    impl BinImpl for CustomSource {}

    impl URIHandlerImpl for CustomSource {
        const URI_TYPE: gst::URIType = gst::URIType::Src;

        fn protocols() -> &'static [&'static str] {
            &["file"]
        }

        fn uri(&self) -> Option<String> {
            let state = self.state.lock().unwrap();

            if let Some(state) = &*state {
                let location: String = state.filesrc.property("location");
                let url = Url::from_file_path(location)
                    .expect("CustomSource::get_uri couldn't build `Url` from `location`");
                
                Some (
                    String::from(url.as_str())
                )
            }
            else {
                gst::error!(CAT, imp: self, "Cannot get uri before state has been built");
                None
            }
        }

        fn set_uri(&self, uri: &str) -> Result<(), glib::Error> {
            let uri = String::from(uri);
            let location = {
                if uri.starts_with("file://") {

                    let url = Url::from_str(uri.as_str()).unwrap();
                    match url.to_file_path() {
                        Ok(file_path) =>  { 
                            let file_path = file_path
                                .as_os_str()
                                .to_str();
                            
                            if let Some(file_path) = file_path {
                                Some(String::from(file_path))
                            }
                            else {
                                None
                            }
                        },
                        Err(_) => None,
                    }
                }
                else {
                    None
                }
            };

            gst::debug!(CAT, imp: self, "Location: {location:?}");

            if let Some(location) = location {
                self.instance().set_property("location", location.as_str());

                Ok(())
            }
            else {
                Err(
                    glib::Error::new(
                        gst::URIError::BadUri, 
                        "Could not set file location")
                )
            }
        }
    }
}