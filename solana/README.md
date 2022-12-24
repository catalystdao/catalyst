# Deploy
- Everything: `anchor deploy`
- Specific Program: `anchor deploy -p <ProgramName>`
    -   Note: `<ProgramName>` is the name of the program in TitleCase
# Wallet
test-wallet.json contains the keypair used for testing of the Anchor programs.
# Add program
Add program key to Anchor.toml, example:
- programName = "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"
# Run test:
Run a specific test:
- `anchor test tests/<Path>/*.ts`
- Anchor.toml test script has been modified to allow running of specific tests. Original code:
    - `test = "yarn run ts-mocha -p ./tsconfig.json -t 1000000 tests/**/*.ts"`