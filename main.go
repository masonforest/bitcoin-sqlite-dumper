package main
import ("fmt"
 "math" 
 "encoding/hex"
 "log"
)

func Varint128Read(bytes []byte, offset int) ([]byte, int) { // take a byte array and return (byte array and number of bytes read)

    // store bytes
    result := []byte{} // empty byte slice

    // loop through bytes
    for _, v := range bytes[offset:] { // start reading from an offset

        // store each byte as you go
        result = append(result, v)

        // Bitwise AND each of them with 128 (0b10000000) to check if the 8th bit has been set
        set := v & 128 // 0b10000000 is same as 1 << 7

        // When you get to one without the 8th bit set, return that byte slice
        if set == 0 {
            return result, len(result)
            // Also return the number of bytes read
        }
    }

    // Return zero bytes read if we haven't managed to read bytes properly
    return result, 0

}

func Varint128Decode(bytes []byte) int64 { // takes a byte slice, returns an int64 (makes sure it work on 32 bit systems)

    // total
    var n int64 = 0

    for _, v := range bytes {

        // 1. shift n left 7 bits (add some extra bits to work with)
        //                             00000000
        n = n << 7

        // 2. set the last 7 bits of each byte in to the total value
        //    AND extracts 7 bits only 10111001  <- these are the bits of each byte
        //                              1111111
        //                              0111001  <- don't want the 8th bit (just indicated if there were more bytes in the varint)
        //    OR sets the 7 bits
        //                             00000000  <- the result
        //                              0111001  <- the bits we want to set
        //                             00111001
        n = n | int64(v & 127)

        // 3. add 1 each time (only for the ones where the 8th bit is set)
        if (v & 128 != 0) { // 0b10000000 <- AND to check if the 8th bit is set
                            // 1 << 7     <- could always bit shift to get 128
            n++
        }

    }

    return n
    // 11101000000111110110

}

func DecompressValue(x int64) int64 {

    var n int64 = 0      // decompressed value

    // Return value if it is zero (nothing to decompress)
    if x == 0 {
        return 0
    }

    // Decompress...
    x = x - 1    // subtract 1 first
    e := x % 10  // remainder mod 10
    x = x / 10   // quotient mod 10 (reduce x down by 10)

    // If the remainder is less than 9
    if e < 9 {
        d := x % 9 // remainder mod 9
        x = x / 9  // (reduce x down by 9)
        n = x * 10 + d + 1 // work out n
    } else {
        n = x + 1
    }

    // Multiply n by 10 to the power of the first remainder
    result := float64(n) * math.Pow(10, float64(e)) // math.Pow takes a float and returns a float

    // manual exponentiation
    // multiplier := 1
    // for i := 0; i < e; i++ {
    //     multiplier *= 10
    // }
    // fmt.Println(multiplier)

    return int64(result)

}

func main() {
	scriptTypeCount := map[string]int{"p2pk":0, "p2pkh":0, "p2sh":0, "p2ms":0, "p2wpkh":0, "p2wsh":0, "p2tr": 0, "non-standard": 0} 
	p2pkaddresses := true
	testnet := true
	fieldsSelected := map[string]bool{"count":true, "txid":true, "vout":true, "height":true, "coinbase":true, "amount":true, "nsize":true, "script":true, "type":true, "address":true}
	xor, err := hex.DecodeString("f3f097f2da3dbdcb58f0aa8871078b97f48b040eb864b786f208c101e3b6bfabd568f6")
	var totalAmount int64 = 0 
	output := map[string]string{}
	if err != nil {
		log.Fatal(err) // Handle the error
	}
	// ---
	// Key
	// ---

	//      430000155b9869d56c66d9e86e3c01de38e3892a42b99949fe109ac034fff6583900
	//      <><--------------------------------------------------------------><>
	//      /                               |                                  \
	//  type                          txid (little-endian)                      index (varint)

	// txid
	// if fieldsSelected["txid"] {
	// 	txidLE := key[1:33] // little-endian byte order

	// 	// txid - reverse byte order
	// 	txid := make([]byte, 0)                 // create empty byte slice (dont want to mess with txid directly)
	// 	for i := len(txidLE) - 1; i >= 0; i-- { // run backwards through the txid slice
	// 		txid = append(txid, txidLE[i]) // append each byte to the new byte slice
	// 	}
	// 	output["txid"] = hex.EncodeToString(txid) // add to output results map
	// }

	// vout
	// if fieldsSelected["vout"] {
	// 	index := key[33:]

	// 	// convert varint128 index to an integer
	// 	vout := Varint128Decode(index)
	// 	output["vout"] = fmt.Sprintf("%d", vout)
	// }

	// -----
	// Value
	// -----

	// Only deobfuscate and get data from the Value if something is needed from it (improves speed if you just want the txid:vout)
	if fieldsSelected["type"] || fieldsSelected["height"] || fieldsSelected["coinbase"] || fieldsSelected["amount"] || fieldsSelected["nsize"] || fieldsSelected["script"] || fieldsSelected["address"] {

		// // Copy the obfuscateKey ready to extend it
		// obfuscateKeyExtended := obfuscateKey[1:] // ignore the first byte, as that just tells you the size of the obfuscateKey

		// // Extend the obfuscateKey so it's the same length as the value
		// for i, k := len(obfuscateKeyExtended), 0; len(obfuscateKeyExtended) < len(value); i, k = i+1, k+1 {
		// 	// append each byte of obfuscateKey to the end until it's the same length as the value
		// 	obfuscateKeyExtended = append(obfuscateKeyExtended, obfuscateKeyExtended[k])
		// 	// Example
		// 	//   [8 175 184 95 99 240 37 253 115 181 161 4 33 81 167 111 145 131 0 233 37 232 118 180 123 120 78]
		// 	//   [8 177 45 206 253 143 135 37 54]                                                                  <- obfuscate key
		// 	//   [8 177 45 206 253 143 135 37 54 8 177 45 206 253 143 135 37 54 8 177 45 206 253 143 135 37 54]    <- extended
		// }

		// // XOR the value with the obfuscateKey (xor each byte) to de-obfuscate the value
		// var xor []byte // create a byte slice to hold the xor results
		// for i := range value {
		// 	result := value[i] ^ obfuscateKeyExtended[i]
		// 	xor = append(xor, result)
		// }

		// -----
		// Value
		// -----

		//   value: 71a9e87d62de25953e189f706bcf59263f15de1bf6c893bda9b045 <- obfuscated
		//          b12dcefd8f872536b12dcefd8f872536b12dcefd8f872536b12dce <- extended obfuscateKey (XOR)
		//          c0842680ed5900a38f35518de4487c108e3810e6794fb68b189d8b <- deobfuscated
		//          <----><----><><-------------------------------------->
		//           /      |    \                   |
		//      varint   varint   varint          script <- P2PKH/P2SH hash160, P2PK public key, or complete script
		//         |        |     nSize
		//         |        |
		//         |     amount (compressesed)
		//         |
		//         |
		//  100000100001010100110
		//  <------------------> \
		//         height         coinbase

		offset := 0

		// First Varint
		// ------------
		// b98276a2ec7700cbc2986ff9aed6825920aece14aa6f5382ca5580
		// <---->
		varint, bytesRead := Varint128Read(xor, 0) // start reading at 0
		offset += bytesRead
		varintDecoded := Varint128Decode(varint)

		if fieldsSelected["height"] || fieldsSelected["coinbase"] {

			// Height (first bits)
			height := varintDecoded >> 1 // right-shift to remove last bit
			output["height"] = fmt.Sprintf("%d", height)

			// Coinbase (last bit)
			coinbase := varintDecoded & 1 // AND to extract right-most bit
			output["coinbase"] = fmt.Sprintf("%d", coinbase)
			fmt.Print("%v", height)
		}

		// Second Varint
		// -------------
		// b98276a2ec7700cbc2986ff9aed6825920aece14aa6f5382ca5580
		//       <---->
		varint, bytesRead = Varint128Read(xor, offset) // start after last varint
		offset += bytesRead
		varintDecoded = Varint128Decode(varint)

		// Amount
		if fieldsSelected["amount"] {
			amount := DecompressValue(varintDecoded) // int64
			output["amount"] = fmt.Sprintf("%d", amount)
			totalAmount += amount // add to stats
		}

		// Third Varint
		// ------------
		// b98276a2ec7700cbc2986ff9aed6825920aece14aa6f5382ca5580
		//             <>
		//
		// nSize - byte to indicate the type or size of script - helps with compression of the script data
		//  - https://github.com/bitcoin/bitcoin/blob/master/src/compressor.cpp

		//  0  = P2PKH <- hash160 public key
		//  1  = P2SH  <- hash160 script
		//  2  = P2PK 02publickey <- nsize makes up part of the public key in the actual script
		//  3  = P2PK 03publickey
		//  4  = P2PK 04publickey (uncompressed - but has been compressed in to leveldb) y=even
		//  5  = P2PK 04publickey (uncompressed - but has been compressed in to leveldb) y=odd
		//  6+ = [size of the upcoming script] (subtract 6 though to get the actual size in bytes, to account for the previous 5 script types already taken)
		varint, bytesRead = Varint128Read(xor, offset) // start after last varint
		offset += bytesRead
		nsize := Varint128Decode(varint) //
		output["nsize"] = fmt.Sprintf("%d", nsize)

		// Script (remaining bytes)
		// ------
		// b98276a2ec7700cbc2986ff9aed6825920aece14aa6f5382ca5580
		//               <-------------------------------------->

		// Move offset back a byte if script type is 2, 3, 4, or 5 (because this forms part of the P2PK public key along with the actual script)
		if nsize > 1 && nsize < 6 { // either 2, 3, 4, 5
			offset--
		}

		// Get the remaining bytes
		script := xor[offset:]

		// Decompress the public keys from P2PK scripts that were uncompressed originally. They got compressed just for storage in the database.
		// Only decompress if the public key was uncompressed and
		//   * Script field is selected or
		//   * Address field is selected and p2pk addresses are enabled.
		// if (nsize == 4 || nsize == 5) && (fieldsSelected["script"] || (fieldsSelected["address"] && *p2pkaddresses)) {
			// script = keys.DecompressPublicKey(script)
		// }

		if fieldsSelected["script"] {
			output["script"] = hex.EncodeToString(script)
		}

		// Addresses - Get address from script (if possible), and set script type (P2PK, P2PKH, P2SH, P2MS, P2WPKH, P2WSH or P2TR)
		// ---------
		if fieldsSelected["address"] || fieldsSelected["type"] {

			var address string                     // initialize address variable
			var scriptType string = "non-standard" // initialize script type

			switch {

			// P2PKH
			case nsize == 0:
				if fieldsSelected["address"] { // only work out addresses if they're wanted
					if testnet == true {
						// address = keys.Hash160ToAddress(script, []byte{0x6f}) // (m/n)address - testnet addresses have a special prefix
					} else {
						// address = keys.Hash160ToAddress(script, []byte{0x00}) // 1address
					}
				}
				scriptType = "p2pkh"
				scriptTypeCount["p2pkh"] += 1

			// P2SH
			case nsize == 1:
				if fieldsSelected["address"] { // only work out addresses if they're wanted
					if testnet == true {
						// address = keys.Hash160ToAddress(script, []byte{0xc4}) // 2address - testnet addresses have a special prefix
					} else {
						// address = keys.Hash160ToAddress(script, []byte{0x05}) // 3address
					}
				}
				scriptType = "p2sh"
				scriptTypeCount["p2sh"] += 1

			// P2PK
			case 1 < nsize && nsize < 6: // 2, 3, 4, 5
				//  2 = P2PK 02publickey <- nsize makes up part of the public key in the actual script (e.g. 02publickey)
				//  3 = P2PK 03publickey <- y is odd/even (0x02 = even, 0x03 = odd)
				//  4 = P2PK 04publickey (uncompressed)  y = odd  <- actual script uses an uncompressed public key, but it is compressed when stored in this db
				//  5 = P2PK 04publickey (uncompressed) y = even

				// "The uncompressed pubkeys are compressed when they are added to the db. 0x04 and 0x05 are used to indicate that the key is supposed to be uncompressed and those indicate whether the y value is even or odd so that the full uncompressed key can be retrieved."
				//
				// if nsize is 4 or 5, you will need to uncompress the public key to get it's full form
				// if nsize == 4 || nsize == 5 {
				//     // uncompress (4 = y is even, 5 = y is odd)
				//     script = decompress(script)
				// }

				scriptType = "p2pk"
				scriptTypeCount["p2pk"] += 1

				if fieldsSelected["address"] { // only work out addresses if they're wanted
					if p2pkaddresses { // if we want to convert public keys in P2PK scripts to their corresponding addresses (even though they technically don't have addresses)

						// NOTE: These have already been decompressed. They were decompressed when the script data was first encountered.
						// Decompress if starts with 0x04 or 0x05
						// if (nsize == 4) || (nsize == 5) {
						//     script = keys.DecompressPublicKey(script)
						// }

						if testnet == true {
							// address = keys.PublicKeyToAddress(script, []byte{0x6f}) // (m/n)address - testnet addresses have a special prefix
						} else {
							// address = keys.PublicKeyToAddress(script, []byte{0x00}) // 1address
						}
					}
				}

			// P2MS
			case len(script) > 0 && script[len(script)-1] == 174: // if there is a script and if the last opcode is OP_CHECKMULTISIG (174) (0xae)
				scriptType = "p2ms"
				scriptTypeCount["p2ms"] += 1

			// P2WPKH
			case nsize == 28 && script[0] == 0 && script[1] == 20: // P2WPKH (script type is 28, which means length of script is 22 bytes)
				// 315,c016e8dcc608c638196ca97572e04c6c52ccb03a35824185572fe50215b80000,0,551005,3118,0,28,001427dab16cca30628d395ccd2ae417dc1fe8dfa03e
				// script  = 0014700d1635c4399d35061c1dabcc4632c30fedadd6
				// script  = [0 20 112 13 22 53 196 57 157 53 6 28 29 171 204 70 50 195 15 237 173 214]
				// version = [0]
				// program =      [112 13 22 53 196 57 157 53 6 28 29 171 204 70 50 195 15 237 173 214]
				// version := script[0]
				program := script[2:]

				// bech32 function takes an int array and not a byte array, so convert the array to integers
				var programint []int // initialize empty integer array to hold the new one
				for _, v := range program {
					programint = append(programint, int(v)) // cast every value to an int
				}

				if fieldsSelected["address"] { // only work out addresses if they're wanted
					if testnet == true {
						// address, _ = bech32.SegwitAddrEncode("tb", int(version), programint) // hrp (string), version (int), program ([]int)
					} else {
						// address, _ = bech32.SegwitAddrEncode("bc", int(version), programint) // hrp (string), version (int), program ([]int)
					}
				}

				scriptType = "p2wpkh"
				scriptTypeCount["p2wpkh"] += 1

			// P2WSH
			case nsize == 40 && script[0] == 0 && script[1] == 32: // P2WSH (script type is 40, which means length of script is 34 bytes; 0x00 means segwit v0)
				// 956,1df27448422019c12c38d21c81df5c98c32c19cf7a312e612f78bebf4df20000,1,561890,800000,0,40,00200e7a15ba23949d9c274a1d9f6c9597fa9754fc5b5d7d45fc4369eeb4935c9bfe
				// version := script[0]
				program := script[2:]

				var programint []int
				for _, v := range program {
					programint = append(programint, int(v)) // cast every value to an int
				}

				if fieldsSelected["address"] { // only work out addresses if they're wanted
					if testnet == true {
						// address, _ = bech32.SegwitAddrEncode("tb", int(version), programint) // testnet bech32 addresses start with tb
					} else {
						// address, _ = bech32.SegwitAddrEncode("bc", int(version), programint) // mainnet bech32 addresses start with bc
					}
				}

				scriptType = "p2wsh"
				scriptTypeCount["p2wsh"] += 1

			// P2TR
			case nsize == 40 && script[0] == 0x51 && script[1] == 32: // P2TR (script type is 40, which means length of script is 34 bytes; 0x51 means segwit v1 = taproot)
				// 9608047,bbc2e707dbc68db35dbada9be9d9182e546ee9302dc0a5cdd1a8dc3390483620,0,709635,2003,0,40,5120ef69f6a605817bc88882f88cbfcc60962af933fe1ae24a61069fb60067045963
				// version := 1
				program := script[2:]

				var programint []int
				for _, v := range program {
					programint = append(programint, int(v)) // cast every value to an int
				}

				if fieldsSelected["address"] { // only work out addresses if they're wanted
					if testnet == true {
						// address, _ = bech32.SegwitAddrEncode("tb", version, programint) // testnet bech32 addresses start with tb
					} else {
						// address, _ = bech32.SegwitAddrEncode("bc", version, programint) // mainnet bech32 addresses start with bc
					}
				}

				scriptType = "p2tr"
				scriptTypeCount["p2tr"] += 1

			// Non-Standard (if the script type hasn't been identified and set then it remains as an unknown "non-standard" script)
			default:
				scriptType = "non-standard"
				scriptTypeCount["non-standard"] += 1

			} // switch

			// add address and script type to results map
			output["address"] = address
			output["type"] = scriptType

		} // if fieldsSelected["address"] || fieldsSelected["type"]

	} // if field from the Value is needed (e.g. -f txid,vout,address)

}
