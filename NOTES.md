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

From the "BBC Microcomputer Service Manual":

> 3.7 Keyboard
>
> The keyboard circuit (Section 9.5) connects via PL 13. A 1 MHz clock signal is 
> fed to a 74LS163 binary counter, the outputs of which are decoded by a 7445 
> decoder driver circuit. These outputs drive the rows of the keyboard matrix, 
> each row being driven in turn. If any key is depressed, the 74LS30 gate will 
> produce an output when that row is strobed and this will interrupt the computer 
> through line CA 2 of IC3. On this interrupt, the computer will enter the key 
> reading software. In order to discover which key was pressed, the microprocessor 
> loads directly into the 74LS163 the address of each key matrix row allowing 
> it to interrogate each row in turn. Also, the microprocessor loads into a 
> 74LS251 data selector, the address of each specific key on that row. ie column 
> addresses. In this way, the microprocessor can interrogate each individual 
> key in turn until it discovers which one was depressed and causing the interrupt. 
> Once read, the keyboard assumes its free running mode.
