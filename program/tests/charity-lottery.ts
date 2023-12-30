import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { CharityLottery } from "../target/types/charity_lottery";
import { SYSVAR_SLOT_HASHES_PUBKEY } from "@solana/web3.js";
import { getAssociatedTokenAddress, getAccount } from "@solana/spl-token"
import { expect } from "chai";

describe("charity-lottery", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.CharityLottery as Program<CharityLottery>;

  const lotteryTicketNumbers = {
    commonNumbers: [1, 2, 3, 4, 5],
    specialNumber: 6,
  }

  const drawNumber = new anchor.BN(1);
  const transferTokenAmount = new anchor.BN(1);

  const charityProject = {
    projectName: "World peace",
    description: "Let's protect the earth and care each other.",
  }

  const [drawNumberRecorderPda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("draw number")],
    program.programId,
  );

  const [prizePoolPda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("prize pool")],
    program.programId,
  );

  const [lotteryTicketPda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("buy lottery ticket"), provider.wallet.publicKey.toBuffer()],
    program.programId,
  );

  const [charityMint] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("charity mint")],
    program.programId,
  )

  const [charityProjectPda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from(charityProject.projectName), provider.wallet.publicKey.toBuffer()],
    program.programId,
  );

  it("Initialize draw number recorder", async () => {
    // Add your test here.
    const tx = await program.methods
      .initializeDrawNumberRecorder()
      .accounts({
        drawNumberRecorder: drawNumberRecorderPda,
      })
      .rpc();
    console.log("Your transaction signature", tx);
  });

  it("Increase draw number", async () => {
    const tx = await program.methods
      .increaseDrawNumber()
      .accounts({
        drawNumberRecorder: drawNumberRecorderPda,
      })
      .rpc();
    console.log("Your transaction signature", tx);
  });

  it("New winning numbers", async () => {
    const drawNumberState = await program.account.drawNumberRecorder.fetch(drawNumberRecorderPda);

    const [winningNumbersPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("winning numbers"), drawNumberState.drawNumber.toArrayLike(Buffer, "le", 8)],
      program.programId,
    );

    const tx = await program.methods
      .newWinningNumbers()
      .accounts({
        winningNumbers: winningNumbersPda,
        drawNumberRecorder: drawNumberRecorderPda,
        recentSlothashes: SYSVAR_SLOT_HASHES_PUBKEY,
      })
      .rpc();

    console.log("Your transaction signature", tx);
  });

  it("Initialize prize pool", async () => {
    const tx = await program.methods
      .initializePrizePool()
      .accounts({
        prizePool: prizePoolPda,
      })
      .rpc();
    console.log("Your transaction signature", tx);
  });

  it("Initialize charity mint", async () => {
    const tx = await program.methods
      .initializeCharityMint()
      .rpc()
    console.log("Your transaction signature", tx);
  });

  it("Buy lottery ticket and get charity token", async () => {
    const assTokenAccount = await getAssociatedTokenAddress(
      charityMint,
      provider.wallet.publicKey
    )

    const tx = await program.methods
      .buyLotteryTicket(lotteryTicketNumbers.commonNumbers, lotteryTicketNumbers.specialNumber)
      .accounts({
        lotteryTicket: lotteryTicketPda,
        guide: provider.wallet.publicKey,
        prizePool: prizePoolPda,
        assTokenAccount: assTokenAccount,
        drawNumberRecorder: drawNumberRecorderPda,
      })
      .rpc();

    const account = await program.account.lotteryTicket.fetch(lotteryTicketPda);
    expect(lotteryTicketNumbers.commonNumbers === account.commonNumbers);
    expect(lotteryTicketNumbers.specialNumber === account.specialNumber);

    const buyerAta = await getAccount(provider.connection, assTokenAccount);
    expect(Number(buyerAta.amount)).to.equal(1 * Math.pow(10, 6))

    console.log("Your transaction signature", tx);
  });

  it("Check the ticket numbers", async () => {
    const [winningNumbersPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("winning numbers"), drawNumber.toArrayLike(Buffer, "le", 8)],
      program.programId,
    );

    const tx = await program.methods
      .checkTicketNumbers(drawNumber)
      .accounts({
        lotteryTicket: lotteryTicketPda,
        winningNumbers: winningNumbersPda,
        prizePool: prizePoolPda,
      })
      .rpc();

    console.log("Your transaction signature", tx);
  });

  it("Publish charity project", async () => {
    const assTokenAccount = await getAssociatedTokenAddress(
      charityMint,
      provider.wallet.publicKey
    )

    const tx = await program.methods
      .publishCharityProject(charityProject.projectName, charityProject.description)
      .accounts({
        charityProject: charityProjectPda,
        projectAsstokenAccount: assTokenAccount,
        charityMint: charityMint,
      })
      .rpc()

    const accout = await program.account.charityProject.fetch(charityProjectPda);
    expect(charityProject.projectName === accout.projectName);
    expect(charityProject.description === accout.description);

    console.log("Your transaction signature", tx);
  });

  it("Support charity project.", async () => {
    const supportAsstokenAccount = await getAssociatedTokenAddress(
      charityMint,
      provider.wallet.publicKey
    )

    const projectAsstokenAccount = await getAssociatedTokenAddress(
      charityMint,
      provider.wallet.publicKey
    )

    const tx = await program.methods
      .supportCharityProject(transferTokenAmount)
      .accounts({
        supporterAsstokenAccount: supportAsstokenAccount,
        projectAsstokenAccount: projectAsstokenAccount,
        projectCreatorAccount: provider.wallet.publicKey,
        charityMint: charityMint,
      })
      .rpc()

    const buyerAta = await getAccount(provider.connection, projectAsstokenAccount);
    expect(Number(buyerAta.amount)).to.equal(1 * Math.pow(10, 6))
    console.log("Your transaction signature", tx);
  });
});
