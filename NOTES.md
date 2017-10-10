Keyboard
===
The keyboard is arranged in a row/col matrix; 7 rows and 10 columns. The row
value appears at `PA4-6` and the column value appears at `PA0-3`. ~~Shift and Ctrl
appear at PA7 maybe?~~

The OS polls the keyboard over `PA` by setting its DDR to `0x7f` (write on bits 0-6,
read on bit 7) and then begins to try every rol/col combination. If it "hears"
a `1` on bit 7 after trying a particular row/col then it determines that this 
key was pressed. I'm unsure why but during this process, the OS periodially 
writes `0x0b` and `0x03` to `CB2`, enabling and disabling keyboard write, 
respectivley.

