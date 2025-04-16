# Features

- Select area and "Format as compact" or "Format as expanded"
- Interface recognition for renaming
- JSON schema validation
- Key sorting
- Validation despite Git conflicts


```yml
## Completion opportunities

# 1. Value
a: b
x:  #
  ^^

- a: b
  x:  #
    ^^

a:  #
  ^^
    #
 ^^^
^

a:
    :   #
 ^^^
     ^^^

a:
  -  #
   ^^
```


```yml
# Comment
x: y

# Comment
a: b
x: y

# Comment
- x

# Comment
- a
- x

# Comment
- x: y
```

```yml
## Reproducing the TOML example

title: TOML Example

owner:
  name: Tom
  birth_date: ...

database:
  enabled: true
  ports: [8000, 8001, 8002]
  data: [[delta, phi], 3.14]
  temp_targets: { cpu: 79, case: 72 }

servers.alpha:
  ip: 10.0.0.1
  role: frontend

servers.beta:
  ip: 10.0.0.2
  role: backend
```

```yml
foo:
    # Comment on list item
    - bar: 5
      baz: 3

      # Comment on map entry
    - qux: 7


a:
  # A
  - x
  - y

# A
- x
- y

# A
-
  - x
  - y


- e
# X
-
  # Y
  - a
  - b
```


```yml
# Root

x:

x: y

- x: y

- x:

-


# Map

a:
  - x

a:
  -
    - x

a:
  - x: y

a:
  x: y

a:
  x:

a: b
x:

a: b
x: y


# List

- a
- x: y

- a
- x:

- a
-
```
