# birb-run
Next level task runner (attempt)

> [!IMPORTANT]
> This is a WIP and I'm currently only developing it as a
> sub-[`xtask`](https://github.com/matklad/cargo-xtask) for another project ([nm4p](https://github.com/LucaCiucci/nm4p) course final project).

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
