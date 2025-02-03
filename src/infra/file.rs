#[derive(Clone, Debug)]
pub struct Client {}

impl Client {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn read<P: AsRef<std::path::Path>>(
        &self,
        path: P,
    ) -> Option<Result<Vec<u8>, Box<dyn std::error::Error>>> {
        match tokio::fs::read(path).await {
            Ok(content) => Some(Ok(content)),
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    None
                } else {
                    Some(Err(Box::from(err)))
                }
            }
        }
    }
}
