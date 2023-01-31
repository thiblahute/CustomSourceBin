use gst::glib;
use gst::prelude::*;

glib::wrapper! {
    pub struct CustomSource(ObjectSubclass<imp::CustomSource>) @extends gst::Bin, gst::Element, gst::Object, @implements gst::URIHandler;
}

pub fn register(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    gst::Element::register(
        Some(plugin),
        "customsource",
        gst::Rank::Primary + 1,
        CustomSource::static_type(),
    )
}

mod imp {
    use gst::glib;
    use gst::prelude::*;
    use gst::subclass::prelude::*;
    use url::Url;

    use std::str::FromStr;

    use once_cell::sync::Lazy;

    static CAT: Lazy<gst::DebugCategory> = Lazy::new(|| {
        gst::DebugCategory::new(
            "CustomSource",
            gst::DebugColorFlags::empty(),
            Some("Custom Source Bin"),
        )
    });

    #[derive(Debug, Default)]
    pub struct CustomSource { }

    #[glib::object_subclass]
    impl ObjectSubclass for CustomSource {
        const NAME: &'static str = "CustomSource";
        type Type = super::CustomSource;
        type ParentType = gst::Bin;
        type Interfaces = (gst::URIHandler,);
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
            gst::debug!(CAT, "Setting property {:?}: {value:?}", pspec.name());
            match pspec.name() {
                "location" => {
                    let location = value.get::<String>().unwrap();

                    gst::debug!(CAT, imp: self, "Setting filesrc location: {location:?}");
                    self.obj().by_name("filesrc").unwrap().set_property("location", location.as_str());
                },
                _ => (),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            gst::debug!(CAT, "Getting property: {:?}", pspec.name());

            match pspec.name() {
                "location" => {
                    self.obj().by_name("filesrc").unwrap().property::<Option<String>>("location").to_value()
                },
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();


            let filesrc = gst::ElementFactory::make("filesrc").name("filesrc").build().unwrap();
            let identity = gst::ElementFactory::make("identity").build().unwrap();

            obj.add_many(&[&filesrc, &identity]).expect("Could not add elements to bin");
            filesrc.link(&identity).unwrap();

            let templ = obj.pad_template("src").unwrap();
            let srcpad = gst::GhostPad::builder_with_template(&templ, Some("src")).build();

            srcpad.set_target(Some(&identity.static_pad("src").unwrap())).expect("Set ghostpad target failed");

            obj.add_pad(&srcpad).expect("Couldn't add src pad?!");
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
            let ret = if transition != gst::StateChange::NullToReady {
                self.parent_change_state(transition)
            } else {
                gst::warning!(CAT, "FIXME in GstBin - bypassing bin change state to avoid pads deactivation");
                Ok(gst::StateChangeSuccess::Success)
            };

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

            let location: String = self.obj().by_name("filesrc").unwrap().property("location");
            let url = Url::from_file_path(location)
                .expect("CustomSource::get_uri couldn't build `Url` from `location`");

            Some (
                String::from(url.as_str())
            )
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
