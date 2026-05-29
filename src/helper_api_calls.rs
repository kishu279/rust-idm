use reqwest::Client;
use tokio::fs::{self, File};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio_stream::StreamExt;

#[derive(PartialEq)]
pub enum Status {
    Pending,
    Downloading,
    Completed,
    Failed,
}

pub struct ChunkMetadata {
    pub id: u32,
    pub start: u64,
    pub end: u64,
    pub downloaded: u32,
    pub status: Status,
    pub temp_path: String,
}

pub struct DownloadMetadata {
    pub url: String,
    pub file_name: String,
    pub total_size: u64,
    pub content_type: String,
    pub supports_range: bool,

    pub etag: Option<String>,
    // last_modified: Option<String>,
    pub chunks: Vec<ChunkMetadata>,
}

pub async fn header_check(url: &str) -> Result<Option<DownloadMetadata>, reqwest::Error> {
    let client = Client::new();

    let response = client
        .get(url)
        .header("Range", "bytes=0-100")
        .send()
        .await?;

    if response.status().is_success() && response.headers().contains_key("Content-Range") {
        let file_name = url.rsplit('/').next().unwrap_or("download").to_string();
        let content_type = response
            .headers()
            .get("Content-Type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();

        let etag = response
            .headers()
            .get("ETag")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let total_size = response
            .headers()
            .get("Content-Range")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.split('/').nth(1))
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        return Ok(Some(DownloadMetadata {
            url: url.to_string(),
            file_name,
            total_size,
            content_type,
            supports_range: true,
            etag,
            chunks: vec![],
        }));
    }

    Ok(None)
}

pub async fn download_chunks(
    data: &mut DownloadMetadata,
    path: &str,
) -> Result<(), reqwest::Error> {
    let mut handles = vec![];
    let url = data.url.clone();

    fs::create_dir_all(format!("{}", path)).await.unwrap();

    for i in &mut data.chunks {
        // change the status
        if i.status == Status::Pending {
            i.status = Status::Downloading;
        }

        let temp_path = i.temp_path.clone();
        let path = path.to_string();
        let url = url.clone();
        let start = i.start;
        let end = i.end;

        let handle = tokio::spawn(async move {
            let client = Client::new();
            let responsne = client
                .get(url)
                .header("Range", format!("bytes={}-{}", start, end))
                .send()
                .await?;

            if responsne.status().is_success() {
                let file = File::create(format!("{}/{}", path, temp_path))
                    .await
                    .unwrap();
                let mut writer = BufWriter::new(file);
                let mut stream = responsne.bytes_stream();

                while let Some(chunk) = stream.next().await {
                    writer.write_all(&chunk?).await.unwrap();
                }

                writer.flush().await.unwrap();
            }

            Ok::<_, reqwest::Error>(())
        });

        handles.push(handle);
    }

    for handle in handles {
        if let Err(e) = handle.await.unwrap() {
            println!("Error downloading chunk: {}", e);
            return Err(e);
        } else {
            println!("Chunk downloaded successfully");
        }
    }

    for chunk in &mut data.chunks {
        if chunk.status == Status::Downloading {
            chunk.status = Status::Completed;
        }
    }

    Ok(())
}

pub async fn merge_chunks(data: &mut DownloadMetadata, path: &str) -> std::io::Result<()> {
    let output_file = File::create_new(format!("{path}/{}", &data.file_name)).await?;
    let mut writer = BufWriter::new(output_file);

    for chunk in &data.chunks {
        let chunk_file = File::open(format!("./temp/{}", &chunk.temp_path)).await?;

        let mut reader = BufReader::new(chunk_file);

        tokio::io::copy(&mut reader, &mut writer).await?;
    }

    writer.flush().await?;

    for chunk in &data.chunks {
        fs::remove_file(format!("./temp/{}", &chunk.temp_path)).await?;
    }

    fs::remove_dir("./temp").await.ok();

    Ok(())
}
