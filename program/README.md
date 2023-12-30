# Anchor Solana Program

First run `anchor build` to build the program.
Then run `anchor keys sync` to update the program ID of the project.
```shell
anchor build
anchor keys sync
```

To deploy the program run `anchor deploy`.
```shell
anchor deploy
```

To run the tests, first install dependencies:
```shell
yarn install
yarn add ts-mocha
```

Next, run `anchor test` to run the test file.
```shell
anchor test
```

When running `anchor test` locally, Anchor will by default:
- start the local test validator
- build the program
- deploy the program to the local test validator
- run the test file
- stop the local test validator

The program contains the following instructions:
1. initialize
    - initialize `DrawNumber`
    - initialize `WinningNumbers`
    - initialize `PrizePool`
2. buy lottery ticket
    - ticket needs to mark whether the prize has been claimed
3. verify number result
    - get the result and prize
4. use the charity token to support project 
5. publish charity project
6. update the charity project

How to create a new winning numbers according to the draw number?
In the program instruction, use CPI to get the blockheight and hash.