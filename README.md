# birb-run
Next level task runner (attempt)

## Core concepts

- [**task**](#task): A task is a unit of work that can be executed. It can have _dependencies_, _parameters_, and _steps_. `task(args; sources) -> outputs`
- **target**: Is just a task with where the output is the name of the target. `target -> task`

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

- make
- Task
- just
