(function() {var implementors = {};
implementors["actix"] = [{text:"impl&lt;T&gt; <a class=\"trait\" href=\"failure/trait.Fail.html\" title=\"trait failure::Fail\">Fail</a> for <a class=\"enum\" href=\"actix/prelude/enum.SendError.html\" title=\"enum actix::prelude::SendError\">SendError</a>&lt;T&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;T: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Sync.html\" title=\"trait core::marker::Sync\">Sync</a> + 'static,&nbsp;</span>",synthetic:false,types:["actix::address::SendError"]},];
implementors["actix_web"] = [{text:"impl&lt;T&gt; <a class=\"trait\" href=\"failure/trait.Fail.html\" title=\"trait failure::Fail\">Fail</a> for <a class=\"struct\" href=\"actix_web/error/struct.InternalError.html\" title=\"struct actix_web::error::InternalError\">InternalError</a>&lt;T&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;T: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/marker/trait.Sync.html\" title=\"trait core::marker::Sync\">Sync</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/fmt/trait.Debug.html\" title=\"trait core::fmt::Debug\">Debug</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/fmt/trait.Display.html\" title=\"trait core::fmt::Display\">Display</a> + 'static,&nbsp;</span>",synthetic:false,types:["actix_web::error::InternalError"]},];
implementors["failure"] = [];
implementors["trust_dns_proto"] = [{text:"impl <a class=\"trait\" href=\"failure/trait.Fail.html\" title=\"trait failure::Fail\">Fail</a> for <a class=\"struct\" href=\"trust_dns_proto/error/struct.ProtoError.html\" title=\"struct trust_dns_proto::error::ProtoError\">ProtoError</a>",synthetic:false,types:["trust_dns_proto::error::ProtoError"]},];
implementors["trust_dns_resolver"] = [{text:"impl <a class=\"trait\" href=\"failure/trait.Fail.html\" title=\"trait failure::Fail\">Fail</a> for <a class=\"struct\" href=\"trust_dns_resolver/error/struct.ResolveError.html\" title=\"struct trust_dns_resolver::error::ResolveError\">ResolveError</a>",synthetic:false,types:["trust_dns_resolver::error::ResolveError"]},];

            if (window.register_implementors) {
                window.register_implementors(implementors);
            } else {
                window.pending_implementors = implementors;
            }
        
})()
