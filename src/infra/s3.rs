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
        let aws_cfg = Self::make_aws_config(cfg).await;
        Self {
            s3: aws_sdk_s3::Client::new(&aws_cfg),
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
            cfg.aws_access_key_id.unwrap(),
            cfg.aws_secret_access_key.unwrap(),
            None,
        );
        aws_config::from_env()
            .endpoint_url(cfg.aws_endpoint_url.unwrap())
            .region(aws_config::Region::new(cfg.aws_region))
            .credentials_provider(creds)
            .load()
            .await
    }

    pub async fn get_object(
        &self,
        bucket: &str,
        key: &str,
    ) -> Option<Result<Vec<u8>, Box<dyn std::error::Error>>> {
        // https://docs.rs/aws-sdk-s3/latest/aws_sdk_s3/client/struct.Client.html#method.get_object
        // https://docs.rs/aws-sdk-s3/latest/aws_sdk_s3/primitives/struct.ByteStream.html
        match self.s3.get_object().bucket(bucket).key(key).send().await {
            Ok(output) => {
                let mut buffer = output
                    .content_length
                    .map_or_else(Vec::new, |size| Vec::with_capacity(size as usize));
                let mut reader = output.body.into_async_read();
                match tokio::io::copy_buf(&mut reader, &mut buffer).await {
                    Ok(_) => (),
                    Err(err) => return Some(Err(Box::from(err))),
                }
                Some(Ok(buffer))
            }
            Err(sdk_err) => match sdk_err.into_service_error() {
                GetObjectError::NoSuchKey(_) => None,
                err => Some(Err(Box::from(err))),
            },
        }
    }
}
