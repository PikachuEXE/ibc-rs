# Data structure and helper function definitions 

This document defines data types and helper functions used by the relayer logic. 

## Data Types

### Chain

Chain is a data structure that captures relayer's perspective of a given chain and contains all important
information that allows relayer to communicate with a chain. A provider is a Tendermint full node through 
which a relayer read information about the given chain and submit transactions. A relayer maintains a list
of full nodes (*peerList*) as a current provider could be faulty, so it can be replaced by another full node.
For each chain a relayer is connected to, the relayer has a light client that provides the relayer 
access to the trusted headers (used as part of data verification).     

```go
type Chain {
  chainID      string
  clientID     Identifier
  peerList     List<Pair<Address, uint64>>
  provider     Pair<Address, uint64>
  lc           LightClient   
}
```

### Client state and consensus state

```go
type ClientState {
  chainID                       string
  validatorSet                  List<Pair<Address, uint64>>
  trustLevel                    Rational
  trustingPeriod                uint64
  unbondingPeriod               uint64
  latestHeight                  Height
  latestTimestamp               uint64
  frozenHeight                  Maybe<uint64>
  upgradeCommitmentPrefix       CommitmentPrefix
  upgradeKey                    []byte
  maxClockDrift                 uint64
  proofSpecs                    []ProofSpec
}
```

```go
type ConsensusState {
  timestamp           uint64
  validatorSet        List<Pair<Address, uint64>>
  commitmentRoot      []byte
}
```

### Membership proof

```go
type MembershipProof struct {
    Height          Height
    Proof           Proof	
}
```

### Connection

```go
type ConnectionEnd {
    state                               ConnectionState
    counterpartyConnectionIdentifier    Identifier
    counterpartyPrefix                  CommitmentPrefix
    clientIdentifier                    Identifier
    counterpartyClientIdentifier        Identifier
    version                             []string
}

enum ConnectionState {
  INIT,
  TRYOPEN,
  OPEN,
}
```

### Channel

```go
type ChannelEnd {
    state                           ChannelState
    ordering                        ChannelOrder
    counterpartyPortIdentifier      Identifier
    counterpartyChannelIdentifier   Identifier
    connectionHops                  [Identifier]
    version                         string
}

enum ChannelState {
  INIT,
  TRYOPEN,
  OPEN,
  CLOSED,
}

enum ChannelOrder {
  ORDERED,
  UNORDERED,
}
```

```go
type Packet {
    sequence           uint64
    timeoutHeight      Height
    timeoutTimestamp   uint64
    sourcePort         Identifier
    sourceChannel      Identifier
    destPort           Identifier
    destChannel        Identifier
    data               []byte	
}
```

```go
type PacketRecv {
     packet          Packet
     proof           CommitmentProof
     proofHeight     Height
}
```

```go
type PacketAcknowledgement {
     packet           Packet
     acknowledgement  byte[]
     proof            CommitmentProof
     proofHeight      Height
}
```

## Helper functions

We assume the existence of the following helper functions:

```go
// Returns channel end with a commitment proof. 
GetChannel(chain Chain, 
           portId Identifier, 
           channelId Identifier,  
           proofHeight Height) (ChannelEnd, CommitmentProof)
 
// Returns connection end with a commitment proof. 
GetConnection(chain Chain, 
              connectionId Identifier, 
              proofHeight Height) (ConnectionEnd, CommitmentProof)


// Returns client state with a commitment proof. 
GetClientState(chain Chain, 
               clientId Identifier, 
               proofHeight Height) (ClientState, CommitmentProof)

// Returns consensus state with a commitment proof. 
GetConsensusState(chain Chain, 
                  clientId Identifier, 
                  targetHeight Height,
                  proofHeight Height) (ConsensusState, CommitmentProof)


// Returns packet commitment with a commitment proof. 
GetPacketCommitment(chain Chain, 
                    portId Identifier, 
                    channelId Identifier, 
                    sequence uint64, 
                    proofHeight Height) (bytes, CommitmentProof)

// Returns next recv sequence number with a commitment proof. 
GetNextSequenceRecv(chain Chain, 
                    portId Identifier, 
                    channelId Identifier,  
                    proofHeight Height) (uint64, CommitmentProof)


// Returns next recv sequence number with a commitment proof. 
GetNextSequenceAck(chain Chain, 
                   portId Identifier, 
                   channelId Identifier,  
                   proofHeight Height) (uint64, CommitmentProof)


// Returns packet acknowledgment with a commitment proof. 
GetPacketAcknowledgement(chain Chain, 
                         portId Identifier, 
                         channelId Identifier, 
                         sequence uint64, 
                         proofHeight Height) (bytes, CommitmentProof)


// Returns packet receipt with a commitment proof. 
GetPacketReceipt(chain Chain, 
                 portId Identifier, 
                 channelId Identifier, 
                 sequence uint64, 
                 proofHeight Height) (String, CommitmentProof)

 
// Returns estimate of the consensus height on the given chain. 
GetConsensusHeight(chain Chain) Height

// Returns estimate of the current time on the given chain. 
GetCurrentTimestamp(chainB) uint64

// Verify that the data is written at the given path using provided membership proof and the root hash. 
VerifyMembership(rootHash []byte, 
                 proofHeight Height, 
                 proof MembershipProof, 
                 path String,
                 data []byte) boolean

// Create IBC datagram as part of processing event at chainA.
CreateDatagram(ev IBCEvent, 
               chainA Chain, 
               chainB Chain, 
               installedHeight Height) (IBCDatagram, Error)

// Create UpdateClient datagrams from the list of signed headers
CreateUpdateClientDatagrams(shs []SignedHeader) IBCDatagram[]

// Submit given datagram to a given chain 
Submit(chain Chain, datagram IBCDatagram) Error 

// Return the correspondin chain for a given chainID 
// We assume that the relayer maintains a map of known chainIDs and the corresponding chains.                
GetChain(chainID String) Chain
```

For functions that return proof, if proof != nil, then the returned value is being verified. 
The value is being verified using the header's app hash that is provided by the corresponding light client.

### Error handling

Helper functions listed above assume querying (parts of the) application state using Tendermint RPC. For example,
`GetChannel` relies on `QueryChannel`. RPC calls can fail as 

- no response is received within some timeout or
- malformed response is received.

In both cases, error handling logic should be defined by the caller. For example, in the former case, the caller might
retry sending the same request to a same provider (full node), while in the latter case the request might be sent to 
some other provider node. Although these kinds of errors could be due to network infrastructure issues, it is normally
simpler to blame the provider (assume implicitly network is always correct and reliable). Therefore, correct provider
always respond timely with a correct response, while in case of errors we consider the provider node faulty, and then 
we replace it with a different node. 

We now show the pseudocode for one of those functions that contains simplified error handling logic:

```go
func GetChannel(chain Chain, 
           portId Identifier, 
           channelId Identifier,  
           proofHeight Height) (ChannelEnd, CommitmentProof) {

    while(true) {
        // Query provable store exposed by the full node of chain. 
        // The path for the channel end is at channelEnds/ports/{portId}/channels/{channelId}".
        // The membership proof returned is read at height proofHeight. 
        channel, proof, error = QueryChannel(chain.provider, portId, channelId, proofHeight) 
        if error != nil {
            // elect a new provider from the peer list
            if !ReplaceProvider(chain) { return (nil, nil) }  // return if fail to elect new provider         
        }
    
        header, error = GetHeader(chain.lc, proofHeight) // get header for height proofHeight using light client
        if error != nil { return (nil, nil) }  // return if light client can't provide header for the given height       

        // verify membership of the channel at path channelEnds/ports/{portId}/channels/{channelId} using 
        // the root hash header.AppHash
        if VerifyMembership(header.AppHash, proofHeight, proof, channelPath(portId, channelId), channel) {
            return (channel, proof)
        } 
        
        // membership check fails; therefore provider is faulty. Try to elect new provider
        if !ReplaceProvider(chain) { return (nil, nil) }  // if fails to elect new provider return
    }
    panic // should never reach this line
}

// Simplified version of logic for electing new provider. In reality it will probably involve opening a connection to 
// a newply elected provider and closing connection with an old provider.
func ReplaceProvider(chain Chain) boolean {
    if chain.peerList.IsEmpty() return false
    chain.provider = Head(chain.peerList)
    chain.peerList = Tail(chain.peerList)
    return true
}
```
If *LATEST_HEIGHT* is passed as a parameter, the data should be read (and the corresponding proof created) 
at the most recent height. 