MAKEFLAGS += --warn-undefined-variables
SHELL := /bin/bash -e -u -o pipefail
AWS_ENDPOINT_URL := http://127.0.0.1:4567
AWS_REGION := ap-northeast-1
AWS_CMD_ENV += AWS_ACCESS_KEY_ID=AAAAAAAAAAAAAAAAAAAA
AWS_CMD_ENV += AWS_SECRET_ACCESS_KEY=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
AWS_CMD_OPT += --endpoint-url=${AWS_ENDPOINT_URL}
AWS_CMD_OPT += --region=${AWS_REGION}
AWS_CMD := ${AWS_CMD_ENV} aws ${AWS_CMD_OPT}
AWS_S3_BUCKET_NAME := local-test

create-s3-bucket:
	@${AWS_CMD} s3api create-bucket \
		--bucket=${AWS_S3_BUCKET_NAME} \
		--create-bucket-configuration LocationConstraint=${AWS_REGION}

clean-s3-bucket:
	@${AWS_CMD} s3 rm s3://${AWS_S3_BUCKET_NAME} --include='*' --recursive

list-s3-bucket:
	@${AWS_CMD} s3 ls s3://${AWS_S3_BUCKET_NAME}/${FOLDER}

copy-object:
	@${AWS_CMD} s3 cp ${SRC} s3://${AWS_S3_BUCKET_NAME}/${DEST}

build-image:
	@docker build -t fanlin-rs:latest .

run-image: fanlin.json
	@cat $^ | jq -c . | xargs -0 docker run --rm --name=fanlin-rs -p 3000:3000 fanlin-rs -j
