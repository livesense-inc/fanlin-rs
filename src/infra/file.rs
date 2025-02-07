#[derive(Clone, Debug)]
pub struct Client {}

impl Client {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn read<P: AsRef<std::path::Path>>(
        &self,
        path: P,
    ) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
        match tokio::fs::read(path).await {
            Ok(content) => Ok(Some(content)),
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    Ok(None)
                } else {
                    Err(Box::from(err))
                }
            }
        }
    }
}

#[cfg(test)]
impl Client {
    pub fn for_test() -> Self {
        Self::new()
    }
}
