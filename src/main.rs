use clap::Parser;
use std::cmp;

mod helper_api_calls;
use helper_api_calls::*;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(short, long)]
    url: String,

    #[arg(short, long, default_value_t = 8)]
    conn: u32,

    #[arg(short, long, default_value = "./temp")]
    path: String,
}

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let args = Args::parse();

    let url = args.url;
    let conn = args.conn;
    let path = args.path;

    // let url = "https://vikingfile.com/d/YRmdoj5iTl/Spider-Noir.S01E01.1080p.AMZN.WEB-DL.DUAL.DDP5.1.H.264-.mkv";
    // // "https://releases.ubuntu.com/26.04/ubuntu-26.04-desktop-amd64.iso";

    // constants
    // let connections = 10; // number of connections

    let response = helper_api_calls::header_check(&url[0..]).await;

    match response {
        Ok(Some(mut data)) => {
            let each_chunk = data.total_size / conn as u64;
            println!("chunk created");
            create_chunk(&mut data, conn as u64, each_chunk);

            // download chunks asynchronously
            download_chunks(&mut data, &path[0..]).await?;
            println!("chunks downloaded");

            // merge the filese
            let _ = merge_chunks(&mut data, &path[0..]).await;
            println!("files deleted and merge chunks done");
        }
        Ok(None) => {
            println!("Failed to fetch headers.");
        }
        Err(error) => {
            println!("An error occurred: {}", error);
        }
    }

    Ok(())
}

// handler to just create the chunk
fn create_chunk(data: &mut DownloadMetadata, connections: u64, each_chunk_size: u64) {
    for i in 0..connections {
        let start = i * each_chunk_size;
        let end = cmp::min((i + 1) * each_chunk_size - 1, data.total_size - 1);

        data.chunks.push(ChunkMetadata {
            downloaded: 0,
            start,
            end,
            id: i as u32,
            status: Status::Pending,
            temp_path: format!("chunk_{i}_tmp"),
        });
    }
}
