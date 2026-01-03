# Todotree: Visualize Tasks with a Dependency Tree

Todotree visualize tasks as a dependency tree rather than a flat list, highlighting complex relationships and color-coding their statuses. Inspired by the structure of Makefiles and the readability of Markdown.

[Watch the demo on YouTube](https://github.com/user-attachments/assets/6590eba5-d053-4c93-9675-724348108536)

- **Dependency Tree**: Todos are displayed in a tree format, showing dependencies between tasks.
- **Actionable Todos**: Tasks that are actionable are highlighted in **red**, making them easy to spot.
- **Pending Todos**: Tasks that are not actionable yet.
- **Completed Todos**: Completed tasks are marked in **blue**, if they are taged with \~ in the input markdown file.
- **Multiple Output Formats**: Supports output in terminal, html, json and markdown formats.

Todotree automatically categorizes your tasks as **Pending** or **Actionable** (red) unless they're marked as **Completed** (using `~` or enclosed in `~~`).


## Example 
#### Input 
[todotree.md](examples/todotree.md) 

#### Output
- Terminal
![terminal](examples/todotree.png "Title")
- [html](https://htmlpreview.github.io/?https://raw.githubusercontent.com/daimh/todotree/refs/heads/master/examples/output/todotree.html)
- [md](examples/output/todotree.md)
- [json](examples/output/todotree.json)


## Installation

Clone the repo and go to the directory
```sh
cargo build --release
cp target/release/todotree ~/bin # or any directory in your PATH
```

## Usage
- show the todo tree in "examples/todotree.md"
```
cd examples
todotree 
```

- hide the todos that were completed
```
todotree -n
```

- show some specific todo
```
todotree lawn garden
```

- Show todos up to 1 layer from root
```
todotree -d 1
```

- Show todos until 1 layer to leaf
```
todotree -d -1
```

- merge lines with some string other than "\n"
```
todotree -i no-owner.md
todotree -i no-owner.md -s "; "
```

- save the output as a file
```
todotree -o term -i name-only.md > name-only.term && cat name-only.term
```

- show the output on the fly while editing
```
todotree -r
```

- other formats
```
todotree -o html -i no-comment.md > no-comment.html
todotree -o json -i no-owner.md > no-owner.json
todotree -o md -i no-owner.md > no-owner-new.md
```

- run the executable md file
```
./name-only.md
./no-comment.md
```

- help
```
todotree -h
```

- compile statically linked exectuable
```
RUSTFLAGS="-C target-feature=+crt-static" cargo build --release
```

## Todo Format for Markdown Input

Each to-do item in the input markdown file is defined by four special lines:

1. **`# <todo-name>`**: The task name, consisting of alphabets, digits, and some special characters. Completed tasks will be displayed in **blue** in both the output HTML file and terminal. To mark a task as completed, prefix it with `~` or enclose it in `~~`, which will also apply a strikethrough style in the markdown file.
   
2. **`- @ <owner>`**: The optional owner of the to-do. This field allows you to assign responsibility to a specific person or team.
   
3. **`- : <dependencies>`**: An optional list of dependencies for the to-do, which can span multiple lines for easier editing. This allows you to track tasks that must be completed before others.
   
4. **`- % <comment>`**: An optional comment or note related to the to-do, providing additional context or details. It can span multiple lines too


## License
The MIT License

## Contributing
Feel free to send me a pull/feature request
