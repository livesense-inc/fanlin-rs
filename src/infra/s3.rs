use aws_config;
use aws_credential_types::Credentials;
use aws_sdk_s3;
use aws_sdk_s3::operation::get_object::GetObjectError;

pub struct Client {
    s3: aws_sdk_s3::Client,
}

impl Client {
    pub async fn new(endpoint: &str, region: &str, access_key: &str, secret_key: &str) -> Self {
        let aws_cfg = Self::make_aws_config(endpoint, region, access_key, secret_key).await;
        Self {
            s3: aws_sdk_s3::Client::new(&aws_cfg),
        }
    }

    async fn make_aws_config(
        endpoint: &str,
        _region: &str,
        access_key: &str,
        secret_key: &str,
    ) -> aws_config::SdkConfig {
        // FIXME: region string has &'static str lifetime
        if endpoint.len() == 0 {
            return aws_config::from_env().region("ap-northeast-1").load().await;
        }

        let creds = Credentials::from_keys(access_key, secret_key, None);
        aws_config::from_env()
            .endpoint_url(endpoint)
            .region("ap-northeast-1")
            .credentials_provider(creds)
            .load()
            .await
    }

    pub async fn get_object<'a>(
        &self,
        bucket: &'static str,
        key: &'a str,
    ) -> Option<Result<Vec<u8>, Box<dyn std::error::Error>>> {
        // https://docs.rs/aws-sdk-s3/latest/aws_sdk_s3/client/struct.Client.html#method.get_object
        // https://docs.rs/aws-sdk-s3/latest/aws_sdk_s3/primitives/struct.ByteStream.html
        match self.s3.get_object().bucket(bucket).key(key).send().await {
            Ok(output) => {
                let mut buffer = output
                    .content_length
                    .map_or_else(|| Vec::new(), |size| Vec::with_capacity(size as usize));
                let mut reader = output.body.into_async_read();
                match tokio::io::copy_buf(&mut reader, &mut buffer).await {
                    Ok(_) => (),
                    Err(err) => return Some(Err(Box::from(err))),
                }
                return Some(Ok(buffer));
            }
            Err(sdk_err) => match sdk_err.into_service_error() {
                GetObjectError::NoSuchKey(_) => None,
                err @ _ => return Some(Err(Box::from(err))),
            },
        }
    }
}
