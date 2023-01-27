use gst::{
    prelude::{Cast, PluginFeatureExtManual},
    traits::{ElementExt, GstObjectExt},
};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        println!("Usage: {} <PATH>", args[0]);
        return;
    }

    let location = String::from(&args[1]);

    gst::init().expect("Could not init GStreamer");

    let filesrc_factory = gst::ElementFactory::find("filesrc").expect("Could not find filesrc factory");
    filesrc_factory.set_rank(filesrc_factory.rank() - 1);

    let customsource_factory = gst::ElementFactory::find("customsource").expect("Could not find CustomSource factory");
    customsource_factory.set_rank(customsource_factory.rank() + 1);

    let pipeline = format!("uridecodebin uri=\"file:///{}\" name=decoder ! autovideosink decoder. ! audioresample ! audioconvert ! autoaudiosink", location);

    println!("Pipeline: {pipeline:?}");

    let pipeline = gst::parse_launch(pipeline.as_str())
        .expect("Could not parse pipeline description");
    
    let pipeline = pipeline.downcast::<gst::Pipeline>().unwrap();

    pipeline
        .set_state(gst::State::Playing)
        .expect("Could not set pipeline to Playing state");

    let bus = pipeline.bus().unwrap();
    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        use gst::MessageView;

        match msg.view() {
            MessageView::Error(err) => {
                eprintln!(
                    "Error received from element {:?} {}",
                    err.src().map(|s| s.path_string()),
                    err.error()
                );
                eprintln!("Debugging information: {:?}", err.debug());
                break;
            }
            MessageView::StateChanged(state_changed) => {
                if state_changed.src().map(|s| s == pipeline).unwrap_or(false) {
                    println!(
                        "Pipeline state changed from {:?} to {:?}",
                        state_changed.old(),
                        state_changed.current()
                    );
                }
            }
            MessageView::Eos(..) => {
                println!("Received EOS");
                break;
            }
            _ => (),
        }
    }

    pipeline
        .set_state(gst::State::Null)
        .expect("Could not set pipeline to Null state");
    println!("Done");
}