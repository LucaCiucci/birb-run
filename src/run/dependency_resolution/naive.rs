
/*
fn build_dependency_list<'a>(
    graph: &'a HashMap<TaskInvocation, LinkedHashSet<TaskInvocation>>,
    top: &'a TaskInvocation,
) -> LinkedHashSet<&'a TaskInvocation> {
    let mut list = LinkedHashSet::new();

    struct StackElement<'a> {
        invocation: &'a TaskInvocation,
        iter: linked_hash_set::Iter<'a, TaskInvocation>,
    }

    let mut visiting = HashSet::new();
    let mut stack = Vec::new();

    stack.push(StackElement {
        invocation: top,
        iter: graph.get(top).unwrap().iter(),
    });

    while let Some(task) = stack.last_mut() {
        let invocation = task.invocation;

        // FIXME this is done multiple times: for each dependency there will
        // be and iteration and these two lines will be executed over and over for
        // the same task.
        visiting.insert(invocation.clone());
        list.insert_if_absent(invocation);

        if let Some(dep) = task.iter.next() {
            todo!()
        } else {
            todo!()
        }
    }

    list
}
*/