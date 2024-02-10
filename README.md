# marksage

A CLI markdown pretty printer that can also run AST transformations. Runs fast enough to stay out of your way. Run it on a schedule as a systemd unit, or while working on markdown tables to pretty print and align them!

```
> marksage  --help
Usage: marksage [OPTIONS] --vault-path <VAULT_PATH> <COMMAND>

Commands:
  archive           Archive todos that have been entirely completed
  format            Apply basic formatting to all markdown files in the vault
  notify-conflicts  Use ntfy.sh to send a push notification about sync conflicts
  help              Print this message or the help of the given subcommand(s)

Options:
  -v, --vault-path <VAULT_PATH>  The path to the obsidian vault to operate on
  -d, --dry-run                  Print what would be done without actually doing it
  -h, --help                     Print help
  -V, --version                  Print version
```
```
> marksage notify-conflicts --help
Use ntfy.sh to send a push notification about sync conflicts

Usage: marksage --vault-path <VAULT_PATH> notify-conflicts [OPTIONS] --topic <TOPIC>

Options:
  -n, --ntfy-url <NTFY_URL>  The ntfy.sh url to send the notification to [default: https://ntfy.sh]
  -t, --topic <TOPIC>        The topic to send the notification to
  -h, --help                 Print help
  ```

## Example

```
---
frontmatter: "frontmatter"
---


#todo

- [x] done
- [x] also done
- [x] top level
    - [x] nested
    - [ ] nested not done
- [ ] not done
- [x] totally
    - [x] done

Some stuff, like text, with ``code``!

header
---


| Tables | Are | Cool |
| --: | :--: | --- |
| left | center | deƒault |
| smol | looooooooong | smol |
| ƒooo | <- unicode | ƒooo |


```
![image](https://github.com/isaec/marksage/assets/72410860/8f49e9d2-a0fb-455f-b5a6-d0f0d1f2d38b)

`marksage --vault-path testing-markdown/ format`

```
---
frontmatter: "frontmatter"
---

#todo

- [x] done
- [x] also done
- [x] top level
    - [x] nested
    - [ ] nested not done
- [ ] not done
- [x] totally
    - [x] done

Some stuff, like text, with `code`!

## header

| Tables |     Are      | Cool    |
| -----: | :----------: | ------- |
|   left |    center    | deƒault |
|   smol | looooooooong | smol    |
|   ƒooo |  <- unicode  | ƒooo    |
```

Alternatively, suppose you ran `archive` because this note is tagged with `#todo` and has tasks on it...

![image](https://github.com/isaec/marksage/assets/72410860/9c44c6b8-2e1a-42d9-bf5a-1ecb8316d159)

`marksage --vault-path testing-markdown/ archive`

```
---
frontmatter: "frontmatter"
---

#todo

- [x] top level
    - [x] nested
    - [ ] nested not done
- [ ] not done

## Archived

- [x] done
- [x] also done
- [x] totally
    - [x] done

Some stuff, like text, with `code`!

## header

| Tables |     Are      | Cool    |
| -----: | :----------: | ------- |
|   left |    center    | deƒault |
|   smol | looooooooong | smol    |
|   ƒooo |  <- unicode  | ƒooo    |
```
And suppose you checked off "nested not done" and ran archive again...

```
---
frontmatter: "frontmatter"
---

#todo

- [ ] not done

## Archived

- [x] top level
    - [x] nested
    - [x] nested not done
- [x] done
- [x] also done
- [x] totally
    - [x] done

Some stuff, like text, with `code`!

## header

| Tables |     Are      | Cool    |
| -----: | :----------: | ------- |
|   left |    center    | deƒault |
|   smol | looooooooong | smol    |
|   ƒooo |  <- unicode  | ƒooo    |
```
