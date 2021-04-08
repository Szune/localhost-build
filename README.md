# localhost-build 0.5.1
A basic build scripting language very quickly thrown together, it's a hacky mess.

At least it was fun to create. (It's still a work in progress though!)

## Installing
1. Clone project
2. Build using cargo build
3. Place directory containing `lb.exe` in the PATH environment variable

## Using in a project
Add a file called `build.lb` in the root directory of the project.

Add some commands to the file or simply import one of the already made build scripts.

## Running build.lb
If you have built `lb.exe`, just run `lb` in the root directory of the project and it will execute the `build.lb` file in that directory.

## Writing build.lb
### Importing
Importing from included build scripts is done by having the following in `build.lbd`:
```
# if you want to use the rust script:
&import(lblib/rust.lb)
```

### Phases

Phases are basically just labels for different parts of the script and are defined as such:
```
# the following line defines the phase @build
@build

# you can use :goto or :gotot to go to the phase
:goto @build
```

### Do X if Y
There are some basic (yes, everything is basic here) commands for evaluating stuff
```
$config-something = sweet
:if $config-something
:eq sour
:and
:if $config-something
:neq sweet
:lt sorry, it was sour
:lf it was probably sweet
# writing this example, I'm not sure why I created this abomination of a script language
:q
```

### Variables
Variables have two types of initialization, "always set" or "set if not already set":
```
# assigning with = will always set
$overwrite-me = hello bird

# assigning with ?= will set if not already set
$overwrite-me ?= hello world

# the following will print "hello bird"
:l $overwrite-me
```

### Comments
Comments are not allowed on the same line as a command (:), i.e. they will simply be arguments to the command:

```
# this is fine

:l logging something # and here's a comment
# the line above will print "logging something # and here's a comment"
```

### Available commands
Add the following to the top of the `build.lb` you're writing to see all the commands:
```
# for verbose help output:
:hv
# for minimal help output:
:h
```

### Extensibility
By using command groups you don't have to repeat yourself quite as much.

Defining a command group:
```
# test-1 is the name of the command group
[test-1 $arg-1
    :l $arg-1
]
```

Calling the command group:
```
!test-1 "hello world"
```

Another example showing how command groups work is `lblib/util.lb` which has the following command group:
```
# go to phase if arg was specified
[gotoarg $arg $goto
    # if the argument in $arg was specified when running lb,
    :hasarg $arg
    # go to the phase in $goto
    :gotot $goto
    # :gotot @build = only goto phase "@build" if last :if result was true
]
```
Which can be used in the following way:

```
# build only
$build = build -b
# if the "only build" argument (above) was specified, go to @build-only
!gotoarg $build @build-only

@build
:l building
# run cargo build
:e cargo build
# quit on error
:qoe

@check
:l analyzing
# run cargo check
:e cargo check
# quit
:q

@build-only
:l building
:e cargo build
:q
```