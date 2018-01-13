# AWatchLog

AWatchLog is an AWS Cloudwatch log agent. 

It is a small daemon use to stream your common file system logfile to AWS Cloudwatch log service.

## Why This Agent?

The official agent provides by AWS is a python program who suffer from high memory usage (> 200MB) and rarely hang up process during the streaming event to Amazon.
The memory leak can be huge, depending on the numbers of files you process at same time.

This agent has on purpose to consume less than 20MB memory and still be able to stream log under high usage.

## Roadmap

The project is still under active development, there are not yet `release candidate` available.
Look at the first milestone to know the progress.

https://github.com/Pierozi/awatchlog/milestone/1

## Test Rust Agent

```
make build
make run
```

## Test Python Agent

The python agent have been warp in `Docker` container.
Go on directroy `tests/python` to build `Dockerfile` to be able to test the
official python agent.

```
// Move into python directory
$ cd tests/python

// Build Container by replacing the XXX by your own values
$ docker build --build-arg="AWS_KEY=XXX" --build-arg="AWS_SECRET="XXX" -f Dockerfile -t pierozi/awatchlog:python ../

// Run the container
$ ./watch

// Launch awslog service and show the log
$ docker exec awatchlog-python-test /bin/bash -c 'service awslogs start && tail -f /var/log/awslogs.log'
```

The watch command return nothing by default.
but the container will write some random data in logfile and the log agent stream it to aws.

## Contribution

I do my best to keep code at a certain quality.
If you are much experimented with Rust and see some design pattern or implementation could be improved, please, I would love to know how to make it better.
In this case feel free to open Issue, or even better, publish change code in a Pull Request.
