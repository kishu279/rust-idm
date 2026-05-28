use std::cmp;

mod helper_api_calls;
use helper_api_calls::*;

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let url = "https://releases.ubuntu.com/26.04/ubuntu-26.04-desktop-amd64.iso";

    // constants
    let connections = 10; // number of connections

    let response = helper_api_calls::header_check(url).await;

    match response {
        Ok(Some(mut data)) => {
            let each_chunk = data.total_size / connections;
            println!("chunk created");
            create_chunk(&mut data, connections, each_chunk);

            // download chunks asynchronously
            download_chunks(&mut data).await?;
            println!("chunks downloaded");

            // merge the filese
            let _ = merge_chunks(&mut data).await;
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
