# to-do list for toaststunt compatibility
This document lists Moor's current state of toaststunt and stunt compatibility.

## Complete
* ftime(), with ftime(1) returning a monotonic floating-point.
* listen(): done upstream, needs testing. Likely not using toaststunt's arguments, listen-tls, etc.

## keywords / errors:
* E_FILE
* E_EXEC
* E_INTRPT (?)

### built-in
* generate_json(), parse_json() - serde
* exec(): does this need a rethink? Running only what's inside of a specifically named directory is good, but not being able to read from stdout while a call is running, or write to stdin, is limiting.
* network-related:
  * open_network_connection()
* File IO...
* sqlite
* encode_binary(), decode_binary()
* object hierarchy "improvements": multiple inheritance, ancestors(), descendants(), parents(), chparents(), isa()
  * investigate existing relational support
* slice(), explode(), sort()
* think about read_http - is it necesary? what about the MOO web server; rust http routing framework hooked into moo?


## database file differences
* WAIFs
* anons (probably not implementing)
* task local data
* "data locality" (v5)?
* mesages and values in tracebacks / errors (done in moor, not sure if the dump format is the same as lambda moo)

## Long-term
* renumber(), reset_max_object() - needed for core extraction
* decide what to do about server options
* force_input()
* task_stack(), toaststunt's finished taskk support or similar (config option / debug switch)
* in-database hooks (status?): checkpoint_started, checkpoint_finished, user_connected user_reconnected, user_disconnected user_client_disconnected, do_out_of_band_command (including telnet), handle_user_error
