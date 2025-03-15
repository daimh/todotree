# Todo Tree

Display todos with a tree of dependencies. Highlight ongoing ones with color, or finished ones with strikethrough.

Example: it prints the screen below with the todotree markdown file [todo.md](examples/todo.md)
![alt text](examples/todo.png "Title")

## Installation

Clone the repo and go to the directory
```sh
cargo build --release
```

## Usage

```sh
cd examples
../target/release/todotree todo.md
../target/release/todotree name-only.md
../target/release/todotree no-comment.md
../target/release/todotree no-owner.md
```

The input markdown file requires four special tags
1. "# ", the todo's name, mandatory. Strikethrough style meant this todo is done
1. "- @ ", the todo's owner, optional
1. "- : ", the todo's dependency list, optional
1. "- % ", the todo's comment, optional


## License

the MIT License

## Contributing

feel free to send me a pull request
