FROM ubuntu:xenial

ARG AWS_KEY
ARG AWS_SECRET

RUN apt-get update && apt-get install -y \ 
		bash \ 
		curl \ 
		less \ 
		groff \ 
		jq \ 
		python \
        cron && \
		mkdir /root/.aws

# Install aws cmd
RUN curl -sS https://bootstrap.pypa.io/get-pip.py | python && \
      pip install https://s3.amazonaws.com/cloudformation-examples/aws-cfn-bootstrap-latest.tar.gz && \
      pip install awscli

# Install Python agent
COPY python/awslogs.conf /etc/awslogs/awslogs.conf

RUN curl -sS https://s3.amazonaws.com/aws-cloudwatch/downloads/latest/awslogs-agent-setup.py > /tmp/awslogssetup.py && \
  python /tmp/awslogssetup.py \
    --region eu-west-1 \
    --non-interactive \
    -c /etc/awslogs/awslogs.conf

# Script to push logs in local file
COPY write /usr/local/bin/write

# Entrypoint overload to catch the ctrl+c and stop signals
COPY run.sh /run.sh
ENTRYPOINT ["/bin/bash", "/run.sh"]

# AWS Credential
RUN echo "aws_access_key_id="$AWS_KEY >> /var/awslogs/etc/aws.conf && \
    echo "aws_secret_access_key="$AWS_SECRET >> /var/awslogs/etc/aws.conf

# Launch the infinite write log
CMD ["/usr/local/bin/write", "/var/log/my-app.log"]

