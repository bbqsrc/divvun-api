use divvun_api::language::grammar::GramcheckOutput;
use divvun_api::language::speller::SpellerResponse;
use divvun_api::server::state::ApiError;

steps!(crate::MyWorld => {
    given "I have loaded `se` grammar and speller files" |world, _step| {
        let grammar_file = "grammar/se.zcheck";
        let mut dir_path = world.config.data_file_dir.clone();
        dir_path.push(grammar_file);
        assert_eq!(dir_path.exists(), true, "{} is not loaded", grammar_file);

        let speller_file = "spelling/se.zhfst";
        let mut dir_path = world.config.data_file_dir.clone();
        dir_path.push(speller_file);
        assert_eq!(dir_path.exists(), true, "{} is not loaded", speller_file);
    };

    when regex r"^I go to the endpoint `([^`]*)`$" |world, matches, _step| {
        let url = format!("http://{}{}", &world.config.addr, matches[1]);
        let body = reqwest::get(&url).unwrap().json().unwrap();

        world.json = body;
    };

    then "I get back a JSON object with available languages and their titles" |world, _step| {
        assert_eq!(&world.json, &json!({"available":{"grammar":{"se": "davvisámegiella"},"speller":{"se":"davvisámegiella"}}}));
    };

    when regex r"^I go to the endpoint `([^`]*)` with appropriate data$" |world, matches, _step| {
        let client = reqwest::Client::new();
        let url = format!("http://{}{}", &world.config.addr, matches[1]);

        match matches[1].as_str() {
            "/speller/se" => {
                let response: SpellerResponse = client.post(&url).json(&json!({"text": "oainá páhkat"})).send().unwrap().json().unwrap();
                world.speller_response = Some(response);
            },
            "/grammar/se" => {
                let response: GramcheckOutput = client.post(&url).json(&json!({"text": "sup  ney"})).send().unwrap().json().unwrap();
                world.grammar_response = Some(response);
            },
            _ => {
                panic!("Unsupported endpoint");
            },
        };
    };

    then "I get back a SpellerResponse with suggestions for each word" |world, _step| {
        let response = &world.speller_response.clone().unwrap();
        assert_eq!(response.text, "oainá páhkat");
        assert_eq!(response.results.len(), 2);

        let oaina_res = &response.results[0];
        assert_eq!(oaina_res.word, "oainá");
        assert_eq!(oaina_res.is_correct, false);
        assert_eq!(oaina_res.suggestions.len() > 3, true);
        assert_eq!(oaina_res.suggestions[0].value, "oaidná");
        assert_eq!(oaina_res.suggestions[0].weight, 18.4326171875);

        let pahkat_res = &response.results[1];
        assert_eq!(pahkat_res.word, "páhkat");
        assert_eq!(pahkat_res.is_correct, true);
        assert_eq!(pahkat_res.suggestions.len() > 3, true);
        assert_eq!(pahkat_res.suggestions[0].value, "dahkat");
        assert_eq!(pahkat_res.suggestions[0].weight, 14.0126953125);
    };

    then regex r"^I get back a GrammarOutput with `([^`]*)` and `([^`]*)` error codes$" (String, String) |world, code0, code1, _step| {
        let response = &world.grammar_response.clone().unwrap();
        assert_eq!(response.text, "sup  ney");

        let errs = &response.errs;
        assert_eq!(errs.len(), 2);

        let err0 = &errs[0];
        assert_eq!(err0.error_text, "sup  ney");
        assert_eq!(err0.start_index, 0);
        assert_eq!(err0.end_index, 8);
        assert_eq!(err0.error_code, code0);
        assert_eq!(err0.description, "Ii leat sátnelisttus");
        assert_eq!(err0.title, "Čállinmeattáhusat");
        assert_ne!(err0.suggestions.len(), 0);

        let err1 = &errs[1];
        assert_eq!(err1.error_code, code1);
        assert_eq!(err1.title, "Sátnegaskameattáhusat");
        assert_ne!(err1.suggestions.len(), 0);
    };

    when regex r"^I go to the endpoint `(/speller/.*)` for not loaded language$" (String) |world, endpoint, _step| {
        let client = reqwest::Client::new();
        let url = format!("http://{}{}", &world.config.addr, endpoint);

        let response: ApiError = client.post(&url).json(&json!({"text": "doesn'tmatter"})).send().unwrap().json().unwrap();
        world.api_error = Some(response);
    };

    when regex r"^I go to the endpoint `(/grammar/.*)` for not loaded language$" (String) |world, endpoint, _step| {
        let client = reqwest::Client::new();
        let url = format!("http://{}{}", &world.config.addr, endpoint);

        let response: ApiError = client.post(&url).json(&json!({"text": "doesn't matter"})).send().unwrap().json().unwrap();
        world.api_error = Some(response);
    };

    then regex r"^I get back an ApiError with the message `([^`]*)`$" (String) |world, message, _step| {
        let error = &world.api_error.clone().unwrap();
        assert_eq!(error.message, message);
    };

    when regex r"^I go to the endpoint `([^`]*)` with an appropriate GraphQL query$" (String) |world, endpoint, _step| {
        let client = reqwest::Client::new();
        let url = format!("http://{}{}", &world.config.addr, endpoint);

        let response: serde_json::value::Value = client.post(&url)
            .json(&json!({
                "query": "query { suggestions(text: \"pákhat\", language: \"se\") {\
                    speller { results { word suggestions { value } } }\
                    grammar { errs { errorText errorCode description } }\
                     } }"}))
            .send().unwrap().json().unwrap();

        world.json = response;
    };

    then "I get back a JSON object with both a Speller and Grammar response" |world, _step| {
        let suggestions = &world.json["data"]["suggestions"];
        assert_ne!(suggestions, &json!(null), "no data or suggestions returned");

        let grammar = &suggestions["grammar"];
        assert_ne!(grammar, &json!(null), "no grammar response");

        let speller = &suggestions["speller"];
        assert_ne!(speller, &json!(null), "no speller response");

        assert_eq!(&json!({
          "errs": [
            {
              "errorText": "pákhat",
              "errorCode": "typo",
              "description": "Ii leat sátnelisttus",
            }
          ]
        }), grammar);

        assert_eq!(&json!({
          "results": [{
            "word": "pákhat",
            "suggestions": [{
                "value": "pakehat"
            },{
                "value": "ákkat"
            },{
                "value": "páhkat"
            },{
                "value": "bákčat"
            },{
                "value": "bákŋat"
            }]
          }]
        }), speller);
    };
});
