import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { RewardPoolMain } from "../target/types/reward_pool_main";
import { assert, expect } from "chai";
import { Keypair, SystemProgram, Connection, Transaction, SYSVAR_RENT_PUBKEY, PublicKey } from "@solana/web3.js";
import BN from "bn.js";
import * as splToken from '@solana/spl-token';
import { findProgramAddressSync } from "@project-serum/anchor/dist/cjs/utils/pubkey";
// Función para realizar el airdrop
async function airdrop(connection, pubkey) {
  const airdropSignature = await connection.requestAirdrop(pubkey, 1e9 * 20); // 1 SOL
  await connection.confirmTransaction(airdropSignature, "confirmed");
}

describe("reward_pool_main", () => {
  const provider = anchor.AnchorProvider.local("http://127.0.0.1:8899");
  anchor.setProvider(provider);

  // const connection = provider.connection;
  const wallet = provider.wallet;
  const program = anchor.workspace.RewardPoolMain as Program<RewardPoolMain>;

  const rewardPoolKp = anchor.web3.Keypair.generate();
  // const taxRecipientKp = anchor.web3.Keypair.generate();
  // const campaignTokenKp = anchor.web3.Keypair.generate();

  const a_to_b_mint_authority = anchor.web3.Keypair.generate();
  const a_to_c_mint_authority = anchor.web3.Keypair.generate();

  const payer = anchor.web3.Keypair.generate();
  const payer2 = anchor.web3.Keypair.generate();
  const signerName = anchor.web3.Keypair.generate();



  let a_to_c_mint;
  let a_to_b_mint;

  let taxRecipientAccount;
  let campaignTokenAccount;

  let usrTokenAccount;

  // const taxRecipientAccount = anchor.web3.Keypair.generate();
  // const campaignTokenAccount = anchor.web3.Keypair.generate();

  // let campaignMint;
  // let campaignTokenAccount;
  // let taxRecipientAccount;


  //========================
  // ititialize
  //========================

  let tokenMint: Keypair;
  beforeEach(async () => {

    // creating the projects mint token for testing
    tokenMint = Keypair.generate();
    const transaction = new Transaction();
    await airdrop(provider.connection, payer.publicKey);

    transaction.add(
      SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        newAccountPubkey: tokenMint.publicKey,
        lamports: 1e9,
        space: 82,
        programId: splToken.TOKEN_PROGRAM_ID
      }),
      splToken.createInitializeMintInstruction(
        tokenMint.publicKey,
        9,
        payer.publicKey,
        null
      )
    );

    await provider.sendAndConfirm(transaction, [payer, tokenMint]);

  })
  it("Initializes the reward pool", async () => {

    let poolPDA = await findProgramAddressSync([Buffer.from("reward_pool"), payer.publicKey.toBytes()], program.programId)[0];
    const poolVault = await splToken.getAssociatedTokenAddress(tokenMint.publicKey,poolPDA,true,splToken.TOKEN_PROGRAM_ID,splToken.ASSOCIATED_TOKEN_PROGRAM_ID);

    await program.methods.initialize().accounts({
      rewardPool: poolPDA,
      poolTokenMint: tokenMint.publicKey,
      poolTokenVault: poolVault,
      user: payer.publicKey,
      associatedTokenProgram:splToken.ASSOCIATED_TOKEN_PROGRAM_ID,
      tokenProgram:splToken.TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId
    }).signers([payer]).rpc().catch(e => console.error(e));

    const tx = new anchor.web3.Transaction();

    // mint some tokens to the depositer
    const depositerTokenAccount = await splToken.createAssociatedTokenAccount(provider.connection,payer,tokenMint.publicKey,payer.publicKey);
    tx.add(
        splToken.createMintToInstruction(
            tokenMint.publicKey,
            depositerTokenAccount,
            payer.publicKey,
            1e9 * 1_000_000,
        ),

    )
    await provider.sendAndConfirm(tx, [payer]).catch(e => console.error(e));
    
    const rewardInfoAccount = await findProgramAddressSync([Buffer.from("reward_info"), payer.publicKey.toBytes()],program.programId)[0];
    
    // deposting 100 tokens into the reward info
    await program.methods.depositReward(tokenMint.publicKey, new anchor.BN(100).mul(new anchor.BN(1e9)),new anchor.BN(0),new anchor.BN(1)).accounts({
      poolTokenMint:tokenMint.publicKey,
      rewardPool:poolPDA,
      depositerTokenAccount: depositerTokenAccount,
      campaignTokenAccount: poolVault,
      rewardInfo:rewardInfoAccount,
      depositer:payer.publicKey,
    }).signers([payer]).rpc()

    let poolVaultAccount = await provider.connection.getAccountInfo(poolVault);
    let poolPDATokenBalance = await splToken.AccountLayout.decode(poolVaultAccount.data);
    expect(poolPDATokenBalance.amount).to.equal(BigInt(100000000000));
    
    // withdrawing reward tokens
    await program.methods.withdrawReward(new anchor.BN(0), new anchor.BN(100).mul(new anchor.BN(1e9))).accounts({
      poolTokenMint: tokenMint.publicKey,
      rewardPool:poolPDA,
      campaignTokenAccount:poolVault,
      rewardInfo:rewardInfoAccount,
      userVault:depositerTokenAccount,
      user:payer.publicKey,
    }).signers([payer]).rpc().catch(e => console.error(e))

    poolVaultAccount = await provider.connection.getAccountInfo(poolVault);
    poolPDATokenBalance = await splToken.AccountLayout.decode(poolVaultAccount.data);
    expect(poolPDATokenBalance.amount).to.equal(BigInt(0));
  
  });


  //========================
  // pause
  //========================


});