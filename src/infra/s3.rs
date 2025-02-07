use super::super::config::s3;
use aws_config;
use aws_credential_types::Credentials;
use aws_sdk_s3;
use aws_sdk_s3::operation::get_object::GetObjectError;

#[derive(Clone, Debug)]
pub struct Client {
    s3: aws_sdk_s3::Client,
}

impl Client {
    pub async fn new(cfg: s3::Config) -> Self {
        let aws_cfg = Self::make_aws_config(cfg.clone()).await;
        let mut aws_s3_cfg = aws_sdk_s3::config::Builder::from(&aws_cfg);
        if cfg.aws_endpoint_url.is_some() {
            aws_s3_cfg = aws_s3_cfg.force_path_style(true);
        };
        Self {
            s3: aws_sdk_s3::Client::from_conf(aws_s3_cfg.build()),
        }
    }

    async fn make_aws_config(cfg: s3::Config) -> aws_config::SdkConfig {
        if cfg.aws_endpoint_url.is_none() {
            return aws_config::from_env()
                .region(aws_config::Region::new(cfg.aws_region))
                .load()
                .await;
        }

        let creds = Credentials::from_keys(
            cfg.aws_access_key_id.expect("aws_access_key_id required"),
            cfg.aws_secret_access_key
                .expect("aws_secret_access_key required"),
            None,
        );
        aws_config::from_env()
            .endpoint_url(cfg.aws_endpoint_url.expect("aws_endpoint_url required"))
            .region(aws_config::Region::new(cfg.aws_region))
            .credentials_provider(creds)
            .load()
            .await
    }

    pub async fn get_object(
        &self,
        bucket: String,
        key: String,
    ) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
        // https://docs.rs/aws-sdk-s3/latest/aws_sdk_s3/client/struct.Client.html#method.get_object
        // https://docs.rs/aws-sdk-s3/latest/aws_sdk_s3/primitives/struct.ByteStream.html
        match self.s3.get_object().bucket(bucket).key(key).send().await {
            Ok(output) => {
                let mut buffer = output
                    .content_length
                    .map_or_else(Vec::new, |size| Vec::with_capacity(size as usize));
                let mut reader = output.body.into_async_read();
                let _ = tokio::io::copy_buf(&mut reader, &mut buffer).await?;
                Ok(Some(buffer))
            }
            Err(sdk_err) => match sdk_err.into_service_error() {
                GetObjectError::NoSuchKey(_) => Ok(None),
                err => Err(Box::from(err)),
            },
        }
    }
}

#[cfg(test)]
impl Client {
    pub async fn for_test() -> Self {
        let cfg = s3::Config {
            aws_region: "ap-northeast-1".to_string(),
            aws_endpoint_url: Some("http://127.0.0.1:4567".to_string()),
            aws_access_key_id: Some("AAAAAAAAAAAAAAAAAAAA".to_string()),
            aws_secret_access_key: Some("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string()),
        };
        Self::new(cfg.clone()).await
    }

    async fn put_object<P: AsRef<std::path::Path>>(
        &self,
        bucket: &String,
        key: &String,
        path: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let body = aws_sdk_s3::primitives::ByteStream::from_path(path).await?;
        let _ = self
            .s3
            .put_object()
            .bucket(bucket)
            .key(key)
            .body(body)
            .send()
            .await?;
        Ok(())
    }

    async fn create_bucket(&self) -> Result<String, Box<dyn std::error::Error>> {
        // https://docs.rs/aws-sdk-s3/latest/aws_sdk_s3/client/struct.Client.html#method.create_bucket
        let var = "";
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time went backwards")
            .as_millis();
        let name = format!("rust-test-{timestamp}-{var:p}");
        let cfg = aws_sdk_s3::types::builders::CreateBucketConfigurationBuilder::default()
            .location_constraint(aws_sdk_s3::types::BucketLocationConstraint::ApNortheast1)
            .build();
        let _ = self
            .s3
            .create_bucket()
            .bucket(&name)
            .create_bucket_configuration(cfg)
            .send()
            .await?;
        Ok(name)
    }

    async fn delete_bucket(&self, name: &String) -> Result<(), Box<dyn std::error::Error>> {
        // https://docs.rs/aws-sdk-s3/latest/aws_sdk_s3/client/struct.Client.html#method.list_objects_v2
        for content in self
            .s3
            .list_objects_v2()
            .bucket(name)
            .send()
            .await?
            .contents()
        {
            // https://docs.rs/aws-sdk-s3/latest/aws_sdk_s3/client/struct.Client.html#method.delete_object
            if let Some(key) = content.key() {
                self.s3.delete_object().bucket(name).key(key).send().await?;
            }
        }
        // https://docs.rs/aws-sdk-s3/latest/aws_sdk_s3/client/struct.Client.html#method.delete_bucket
        let _ = self.s3.delete_bucket().bucket(name).send().await?;
        Ok(())
    }
}

#[cfg(test)]
pub struct BucketManager {
    cli: Client,
    list: Vec<String>,
}

#[cfg(test)]
impl BucketManager {
    pub fn new(cli: Client) -> Self {
        let list = Vec::new();
        Self { cli, list }
    }

    pub async fn create(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        let bucket = self.cli.create_bucket().await?;
        self.list.push(bucket.clone());
        Ok(bucket)
    }

    pub async fn upload_fixture_files(
        &self,
        bucket: &String,
        dir: &str,
        path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for result in std::fs::read_dir(dir)? {
            let dir_entry = result?;
            if dir_entry.file_type()?.is_file() {
                let file = dir_entry.file_name().to_str().unwrap().to_string();
                let key = format!("{path}/{file}");
                self.cli.put_object(bucket, &key, dir_entry.path()).await?;
            }
        }
        Ok(())
    }

    pub async fn clean(&self) -> Result<(), Box<dyn std::error::Error>> {
        for bucket in self.list.iter() {
            self.cli.delete_bucket(bucket).await?;
        }
        Ok(())
    }
}
