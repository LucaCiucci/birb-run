# birb-run
Next level task runner (attempt)

> [!IMPORTANT]
> This is a WIP and I'm currently only developing it as a
> sub-[`xtask`](https://github.com/matklad/cargo-xtask) for another project ([nm4p](https://github.com/LucaCiucci/nm4p) course final project).

This is a draft of a task runner, similar to [`make`](https://www.gnu.org/software/make/), [`just`](https://just.systems/), or [`task`](https://taskfile.dev/). It aims to be a more modern and flexible alternative, with a focus on ease of use and correctness.

[![asciicast](https://asciinema.org/a/59zfSVVv8BzgaPb9FbYi9ByYp.svg)](https://asciinema.org/a/59zfSVVv8BzgaPb9FbYi9ByYp)

## Overview

`birb-run` lets you write _taskfiles_, which are usually YAML files defining tasks and their dependencies, for example:
```yaml
# tasks.yaml
tasks:
  car:
    description: Start the car
    deps:
    - task: engine
    steps:
    - echo "Building car..."
  engine:
    steps:
    - echo "Engine built!"
```
In this example, we defined two tasks: `car` and `engine`.  
Now we can "run" the `start` task
```sh
birb run start
```
we get:
```plain
    build       running...
Car built
    start       running...
Car started
```

See the [`demos/`](./demos/) directory for more elaborate examples.

## Core concepts

- [**task**](#task): a unit of work that can be executed. It can have _dependencies_, _parameters_, and _steps_
- **target**: a _task_ with where the output is the name of the target

### Task

The _task_ is the core concept of birb-run. It is a unit of work that can be executed. It can have dependencies, parameters, and steps.

You can see a task as a function that takes arguments and sources, and produces outputs:
```lean
def Task: Args -> Sources -> Outputs
```

A task can be instantiated with a set of arguments. For example a fully instantiated task could look like this:
```lean
def InstantiatedTask: Sources -> Outputs
```
Fully instantiated tasks can then be used for execution.


## Compatibility

- [ ] make
- [ ] Task
- [ ] just
- [ ] xtask
