# charity-lottery

This is the preject about Charity Lottery. You can buy a lottery ticket with small amounts of sol. And at the same time, the program will refund some tokens for you. You can use these tokens to support the project you admire.

The cost of each lottery ticket is 4 * 10^6 lamports (equal to 0.001 sol) 

Number combinations include 5 common numbers and 1 special number. The common numbers range from 1 to 64. The special number ranges from 1 to 32.
| common number | special number |
| --- | --- |
| 64 | 32 |

| number matching | prize/(10^6 lamports) | probability of winning |
| --- | --- | --- |
| only special number | 8 | 1 : 48.73 |
| 1 common + 1 special | 8 | 1 : 107.2 |
| 2 + 1 | 32 | 1 : 751 |
| 3 + 0 | 32 | 1 : 460 |
| 3 + 1 | 200 | 1 : 14260 |
| 4 + 0 | 200 | 1 : 26680 |
| 4 + 1 | 100,000 | 1 : 827066 |
| 5 + 0 | 200,000 | 1 : 7870464 |
| 5 + 1 | first prize | 1 : 243984384 |