# terrible_rust_shell
An attempt to make a shell in rust.

This was mostly just me trying to hit the ground running with rust without
actually knowing anything.

It supports pipes, quotes vs apostrophes, environment variables.

I wanted to support nested commands, but kind of hit a roadblock. mostly
passing inputs/outputs around in a way that the borrow checker was happy
with.

Now I can't even read it.
