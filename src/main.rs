use anyhow::Result;
use anyhow::anyhow;
use clap::Parser;
use metaflac::Tag;
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::PathBuf;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to music directory
    #[arg(short, long)]
    root: PathBuf,

    /// Reset previously processed files
    #[arg(long)]
    reset: bool,
}

fn get_vorbis_as_string(tags: &mut Tag, key: &str) -> Result<String> {
    Ok(tags
        .get_vorbis(key)
        .ok_or(anyhow!("{key} tag not found"))?
        .next()
        .ok_or(anyhow!("{key} has no value"))?
        .to_string())
}

fn get_track_index(tags: &mut Tag) -> Result<u16> {
    let discnumber: u16 = get_vorbis_as_string(tags, "DISCNUMBER")
        .unwrap_or("0".to_string())
        .parse()?;
    let tracknumber: u16 = get_vorbis_as_string(tags, "TRACKNUMBER")?.parse()?;
    Ok((discnumber * 100) + tracknumber)
}

fn reset_track_number(tags: &mut Tag) -> Result<()> {
    let old: u16 = get_vorbis_as_string(tags, "OLDTRACKNUMBER")?.parse()?;
    let track: u16 = get_vorbis_as_string(tags, "TRACKNUMBER")?.parse()?;
    let title = get_vorbis_as_string(tags, "TITLE")?;

    tags.remove_vorbis("OLDTRACKNUMBER");

    if old == track {
        log::debug!("\"{title}\": TRACKNUMBER needs no change");
        return Ok(());
    }

    log::debug!("\"{title}\": resetting TRACKNUMBER: \"{track}\" -> \"{old}\"");
    tags.set_vorbis("TRACKNUMBER", vec![&format!("{old}")]);

    Ok(())
}

fn update_track_number(tags: &mut Tag, track: u16) -> Result<()> {
    let old: u16 = get_vorbis_as_string(tags, "TRACKNUMBER")?.parse()?;
    let title = get_vorbis_as_string(tags, "TITLE")?;

    tags.set_vorbis("OLDTRACKNUMBER", vec![&format!("{old}")]);

    if old == track {
        log::debug!("\"{title}\": TRACKNUMBER needs no change");
        return Ok(());
    }

    log::debug!("\"{title}\": setting TRACKNUMBER: \"{old}\" -> \"{track}\"");
    tags.set_vorbis("TRACKNUMBER", vec![&format!("{track}")]);

    Ok(())
}

fn reset_artist(tags: &mut Tag) -> Result<()> {
    let allartists = get_vorbis_as_string(tags, "ALLARTISTS")?;
    let artist = get_vorbis_as_string(tags, "ARTIST")?;
    let title = get_vorbis_as_string(tags, "TITLE")?;

    tags.remove_vorbis("ALLARTISTS");

    if allartists == artist {
        log::debug!("\"{title}\": ARTIST needs no change");
        return Ok(());
    }

    log::debug!("\"{title}\": resetting ARTIST: \"{artist}\" -> \"{allartists}\"");
    tags.set_vorbis("ARTIST", vec![&allartists]);

    Ok(())
}

fn update_artist(tags: &mut Tag) -> Result<()> {
    // If the ALBUMARTIST tag is populated, use it to set ARTIST and save the old value
    let albumartist = get_vorbis_as_string(tags, "ALBUMARTIST")?;
    let artist = get_vorbis_as_string(tags, "ARTIST")?;
    let title = get_vorbis_as_string(tags, "TITLE")?;

    tags.set_vorbis("ALLARTISTS", vec![&artist]);

    if albumartist == artist {
        log::debug!("\"{title}\": ARTIST needs no change");
        return Ok(());
    }

    log::info!("\"{title}\": updating ARTIST: \"{artist}\" -> \"{albumartist}\"");
    tags.set_vorbis("ARTIST", vec![&albumartist]);

    Ok(())
}

fn process_album(album: &BTreeMap<u16, String>) -> Result<()> {
    for (i, path) in album.values().enumerate() {
        let mut tags = Tag::read_from_path(path)?;
        if get_vorbis_as_string(&mut tags, "MODIFIED").is_ok() {
            log::debug!("File already processed, skipping \"{path}\"");
            continue;
        }
        update_track_number(&mut tags, (i + 1) as u16)?;
        update_artist(&mut tags)?;
        tags.set_vorbis("MODIFIED", vec!["1"]);
        tags.save()?;
    }
    Ok(())
}

fn reset_album(album: &BTreeMap<u16, String>) -> Result<()> {
    for path in album.values() {
        let mut tags = Tag::read_from_path(path)?;
        if get_vorbis_as_string(&mut tags, "MODIFIED").is_err() {
            continue;
        }
        reset_track_number(&mut tags)?;
        reset_artist(&mut tags)?;
        tags.remove_vorbis("MODIFIED");
        tags.save()?;
    }
    Ok(())
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();

    let mut album = BTreeMap::new();
    for entry in WalkDir::new(args.root)
        .contents_first(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        match entry.metadata() {
            Ok(e) if e.is_dir() => {
                let dirname = entry.path().file_name().expect("failed to get file name");
                log::debug!("Processing {dirname:?}");

                // We've seen all of the tracks, process them and reset
                if args.reset {
                    reset_album(&album)?;
                    log::info!("Reset all tracks in {:?}", dirname);
                } else {
                    process_album(&album)?;
                    log::info!("Processed all tracks in {:?}", dirname);
                }
                album = BTreeMap::new();
            }
            Ok(e) if e.is_file() => {
                let path = entry.path();
                if Some("flac") != path.extension().and_then(OsStr::to_str) {
                    log::warn!("Not a `flac` file: {path:?}");
                    continue;
                }

                let mut tags = Tag::read_from_path(path)?;
                let track_index = match get_track_index(&mut tags) {
                    Ok(index) => index,
                    Err(e) => {
                        log::warn!("Failed to get track index for {path:?}: {e}");
                        continue;
                    }
                };
                let _ = album.insert(track_index, path.display().to_string());
            }
            _ => {}
        }
    }

    Ok(())
}
