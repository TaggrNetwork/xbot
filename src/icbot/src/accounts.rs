use std::collections::HashMap;

pub fn get_accounts<'a>() -> HashMap<&'a str, &'a str> {
    let mut map: HashMap<_, _> = Default::default();

    map.insert(
        "d3e13d4777e22367532053190b6c6ccf57444a61337e996242b1abfb52cf92c8",
        "Binance",
    );
    map.insert(
        "220c3a33f90601896e26f76fa619fe288742df1fa75426edfaf759d39f2455a5",
        "Binance",
    );
    map.insert(
        "935b1a3adc28fd68cacc95afcdec62e985244ce0cfbbb12cdc7d0b8d198b416d",
        "Huobi",
    );
    map.insert(
        "e7a879ea563d273c46dd28c1584eaa132fad6f3e316615b3eb657d067f3519b5",
        "Okex",
    );
    map.insert(
        "4dfa940def17f1427ae47378c440f10185867677109a02bc8374fc25b9dee8af",
        "Coinbase",
    );
    map.insert(
        "a6ed987d89796f921c8a49d275ec7c9aa04e75a8fc8cd2dbaa5da799f0215ab0",
        "Coinbase",
    );
    map.insert(
        "449ce7ad1298e2ed2781ed379aba25efc2748d14c60ede190ad7621724b9e8b2",
        "Coinbase",
    );
    map.insert(
        "660b1680dafeedaa68c1f1f4cf8af42ed1dfb8564646efe935a2b9a48528b605",
        "Coinbase",
    );
    map.insert(
        "dd15f3040edab88d2e277f9d2fa5cc11616ebf1442279092e37924ab7cce8a74",
        "Coinbase",
    );
    map.insert(
        "4878d23a09b554157b31323004e1cc053567671426ca4eec7b7e835db607b965",
        "Coinbase",
    );
    map.insert(
        "8fe706db7b08f957a15199e07761039a7718937aabcc0fe48bc380a4daf9afb0",
        "Gate",
    );
    map.insert(
        "efa01544f509c56dd85449edf2381244a48fad1ede5183836229c00ab00d52df",
        "KuCoin",
    );
    map.insert(
        "040834c30cdf5d7a13aae8b57d94ae2d07eefe2bc3edd8cf88298730857ac2eb",
        "Kraken",
    );

    map
}
