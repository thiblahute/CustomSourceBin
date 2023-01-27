use ges::{traits::{TimelineExt, GESPipelineExt, LayerExt, TimelineElementExt}, UriClip};
use gst::{
    prelude::{Cast, ObjectType, PluginFeatureExtManual},
    traits::{ElementExt, GstObjectExt}, ffi::{gst_debug_bin_to_dot_file_with_ts}, glib::translate::ToGlibPtr,
};

fn main() {
    gst::init().expect("Could not init GStreamer");
    ges::init().expect("Could not init GES");

    let filesrc_factory = gst::ElementFactory::find("filesrc").expect("Could not find filesrc factory");
    filesrc_factory.set_rank(filesrc_factory.rank() - 1);

    let customsource_factory = gst::ElementFactory::find("customsource").expect("Could not find CustomSource factory");
    customsource_factory.set_rank(customsource_factory.rank() + 1);

    let args: Vec<String> = std::env::args().collect();
    if !true {
        create_clip(&args[1], 0u64, 5000u64, 0u64);
        
        return;
    }

    if args.len() < 2 {
        println!("Usage: {} <PATH>", args[0]);
        return;
    }

    let timeline = ges::Timeline::new_audio_video();

    // Create a new layer that will contain our timed clips.
    let pipeline = ges::Pipeline::new();
    pipeline.set_timeline(&timeline).expect("Could not add timeline");

    let layer = timeline.append_layer();

     // Create new clips
    let clip = create_clip(&args[1], 0u64, 5000u64, 0u64);
    layer.add_clip(&clip).expect("Could not add clip");

    timeline.commit();

    // let datapullersrc_factory = gst::ElementFactory::find("datapullersrc").expect("Could not find datapullersrc factory");
    // datapullersrc_factory.set_rank(datapullersrc_factory.rank() + 1);

    // let pipeline = format!("uridecodebin uri=\"file:///{}\" name=decoder ! autovideosink decoder. ! audioresample ! audioconvert ! autoaudiosink", location);

    // println!("Pipeline: {pipeline:?}");

    // let pipeline = gst::parse_launch(pipeline.as_str())
    //     .expect("Could not parse pipeline description");
    
    // let pipeline = pipeline.downcast::<gst::Pipeline>().unwrap();

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
                    unsafe {
                        let pipeline = pipeline.clone();

                        gst_debug_bin_to_dot_file_with_ts(
                            pipeline.upcast::<gst::Bin>().as_ptr(), 
                            gst::DebugGraphDetails::ALL.bits(), 
                            "graph".to_glib_none().0
                        );
                    }
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

fn create_clip(path: &str, inpoint: u64, _duration: u64, start: u64) -> UriClip {
    println!("******************************************************");

    let clip = ges::UriClip::new(&path).expect("Failed to create clip");

    println!("======================================================");

    clip.set_inpoint(gst::ClockTime::from_mseconds(inpoint));
    // clip.set_duration(gst::ClockTime::from_mseconds(duration));
    clip.set_start(gst::ClockTime::from_mseconds(start));
    clip.set_name(Some(path)).expect("Could not set clip name");

    return clip;
}