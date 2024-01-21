A tiny systemd service which cycles through the lights, to check that
I've wired them up correctly.  To watch for activity on the buttons, 
use `gpiomon --bias=pull-up 0 2 3 20 21`.
