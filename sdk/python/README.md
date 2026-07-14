# WyrmGrid Python SDK

This zero-dependency SDK implements plugin protocol v1 over standard input and
standard output. WyrmGrid installs a private copy beside each bundled Python
plugin and starts Python in isolated mode.

Plugins receive only capability-approved, stable WyrmGrid messages. They do not
receive OnAir credentials or raw provider responses. Use the bundled
`Fleet Locations` example as the executable starting point.
