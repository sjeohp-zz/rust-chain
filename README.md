

a multiplayer game server is basically a referee

a serverless multiplayer game needs a referee

referee needs to:
- communicate directly with both teams ingame
- mediate transactions
- enforce rules

to choose a referee with least likelihood of being biased:
1) both teams request a referee from the network
2) next node to discover a block nominates a referee and stores the nomination on the blockchain (nodes have an interest in the integrity of the network and therefore the suitability of the referees they nominate)
3) teams either confirm the nomination or veto and request another
4) teams agree on a referee and ruleset

the game is over when a majority of parties (including both teams and the referee) agree the game is over

the end of the game gets recorded on the blockchain along with payment to the referee

if the referee loses contact with the network or the game never ends, the referee doesn't get paid

msg cmds
- gtpr - get peer
- rmpr - remove peer
- lspr - list peers
- gtbl - get balance
- adtx - add transaction
- vdtx - validate transaction
- gtpl - get transaction pool
- gtbs - get blocks
- gtbk - get block
- adbk - add block
- gtht - current block height
- gtlt - get latest block
- chat
- echo
