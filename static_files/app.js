$(document).ready(main);

function main() {
    const SCREEN_AREA = $("#screen_area");

    // See if this is a registration request
    const urlParams = new URLSearchParams(window.location.search);
    const val_code = urlParams.get("val_code");
    if( val_code !== null ) {
        let val_screen = new ValidationScreen();
        val_screen.draw(SCREEN_AREA);
        return;
    }

    // Determine if we already have a valid session id, and if so jump to the main screen
    $.ajax( "/api/v1/validate_session_fe", {
        "method": "GET",
    }).done( function() {
        // Display the main screen
        let main_screen = new MainScreen();
        main_screen.draw(SCREEN_AREA);
    }).fail( function() {
        // Display the login screen
        let login_screen = new LoginScreen();
        login_screen.draw(SCREEN_AREA);
    });
}

function UIScreen() {
    
}

UIScreen.prototype.draw = function (draw_area) {
    let content = $("You drew the parent UIScreen - you should never see this.");
    draw_area.empty();
    draw_area.append(content);
}

function LoginScreen() {
    UIScreen.call(this);
}

LoginScreen.prototype = Object.create(UIScreen.prototype);
Object.defineProperty(LoginScreen.prototype, 'constructor', {
    value: LoginScreen,
    enumerable: false,
    writable: true 
});

LoginScreen.prototype.draw = function (draw_area) {
    let content = $( '<form>' +
        '<label for="login_username">Username:</label>' +
        '<input type="text" id="login_username" autocomplete="username">' +
        '<label for="login_password">Password:</label>' +
        '<input type="password" id="login_password" autocomplete="current-password">' +
        '</form>' );
    let login_button = $( '<input type="button" value="Login">' )
        .click(function() { 
            let login_dat = {
                "username": $("#login_username").val(),
                "password": $("#login_password").val(),
            };
            $.ajax( "/api/v1/authenticate_fe", {
                "method": "POST",
                "data": login_dat,
            }).done(function() {
                // TODO determine if we now have a session id, and if so, jump to the next part, otherwise display failure
                let main_screen = new MainScreen();
                main_screen.draw(draw_area);
            }).fail(function() {
                // TODO determine reason for failure and take action
            });
        });
    let register_button = $( '<input type="button" value="Register">' )
        .click(function() {
            let reg_screen = new RegisterScreen();
            reg_screen.draw(draw_area);
        });

    content.append(login_button);
    content.append(register_button);

    draw_area.empty();
    draw_area.append(content);
}

function RegisterScreen() {
    UIScreen.call(this);
}

RegisterScreen.prototype = Object.create(UIScreen.prototype);
Object.defineProperty(RegisterScreen.prototype, 'constructor', {
    value: RegisterScreen,
    enumerable: false,
    writable: true 
});

RegisterScreen.prototype.draw = function (draw_area) {
    let content = $( '<form>' +
        '<label for="reg_username">Email Address:</label>' +
        '<input type="text" id="reg_username" autocomplete="username">' +
        '<label for="reg_password">Password:</label>' +
        '<input type="password" id="reg_password" autocomplete="current-password">' +
        '<label for="verify_password">Verify Password:</label>' +
        '<input type="password" id="verify_password" autocomplete="current-password">' +
        '</form>'
    );
    let login_button = $( '<input type="button" value="Register">' )
        .click(function() { 
            if( $("#reg_password").val() != $("#verify_password").val() ) {
                // TODO make this nicer
                alert("Passwords do not match!");
                return;
            }
            let reg_dat = {
                "username": $("#reg_username").val(),
                "password": $("#reg_password").val(),
            };
            $.ajax( "/api/v1/create_account", {
                "method": "POST",
                "data": reg_dat,
            }).done(function() {
                // TODO determine whether it was successful and display a message
                alert("User account requested, look for an email...");
                let login_screen = new LoginScreen();
                login_screen.draw(draw_area);
            }).fail(function() {
                // TODO determine reason for failure and take action
            });
        });
    content.append(login_button);

    draw_area.empty();
    draw_area.append(content);
}

function ValidationScreen() {
    UIScreen.call(this);
}

ValidationScreen.prototype = Object.create(UIScreen.prototype);
Object.defineProperty(ValidationScreen.prototype, 'constructor', {
    value: ValidationScreen,
    enumerable: false,
    writable: true 
});

ValidationScreen.prototype.draw = function (draw_area) {
    let content = $( '<form>' +
        '</form>'
    );
    let validate_button = $( '<input type="button" value="Validate Account">' )
        .click(function() {
            const urlParams = new URLSearchParams(window.location.search);
            const val_code = urlParams.get("val_code");
            
            $.ajax( "/api/v1/validate_account?val_code="+val_code, {
                "method": "GET",
            }).done( function() {
                // TODO determine whether it was successful and display a message
                alert("User validation successful!  Now login."); 
                let login_screen = new LoginScreen();
                login_screen.draw(draw_area);
            }).fail( function() {
                // TODO determine reason for failure and take action
            });
        });

    content.append(validate_button);

    draw_area.empty();
    draw_area.append(content);
}

function MainScreen() {
    UIScreen.call(this);
}

MainScreen.prototype = Object.create(UIScreen.prototype);
Object.defineProperty(MainScreen.prototype, 'constructor', {
    value: MainScreen,
    enumerable: false,
    writable: true 
});

MainScreen.prototype.draw = function (draw_area) {
    let content = $( "<div></div>" );
    let button_area = $( "<div></div>" );
    let channel_list_area = $( "<div></div>" );
    let channel_edit_area = $( "<div></div>" );

    draw_channel_list(channel_list_area, channel_edit_area);

    let validate_button = $( '<input type="button" value="Validate Session">' )
        .click(function() {
            $.ajax( "/api/v1/validate_session_fe", {
                "method": "GET",
            }).done( function() {
                alert("Session validation successful."); 
            }).fail( function() {
                alert("Fail");
            });
        });

    let logout_button = $( '<input type="button" value="Logout">' )
        .click(function() {
            $.ajax( "/api/v1/logout_session_fe", {
                "method": "GET",
            }).done( function() {
                let login_screen = new LoginScreen();
                login_screen.draw(draw_area);
            }).fail( function() {
                validate_session_or_login_screen(draw_area);
            });
        });

    let new_channel_list_area = $( "<form>" +
        '<label for="new_channel_list_name">New Channel List Name:</label>' +
        '<input type="textarea" id="new_channel_list_name">' +
        "</form>" );

    let create_channel_list_button = 
        $( '<input type="button" value="Create Channel List">' )
        .click(function() {
            let data = {
                "listname": $("#new_channel_list_name").val(),
            };
            $.ajax( "/api/v1/create_channel_list", {
                "method": "POST",
                "data": data,
            }).done( function() {
                alert("Channel list created."); 
            }).fail( function() {
                alert("Fail");
            });
        });


    new_channel_list_area.append(create_channel_list_button);

    button_area.append(validate_button);
    button_area.append(logout_button);
    button_area.append(new_channel_list_area);

    content.append(channel_list_area);
    content.append(button_area);
    content.append(channel_edit_area);

    draw_area.empty();
    draw_area.append(content);
}

function validate_session_or_login_screen(draw_area) {
    $.ajax( "/api/v1/validate_session_fe", {
        "method": "GET",
    }).done( function() {
        // Do nothing - we're still valid
    }).fail( function() {
        let login_screen = new LoginScreen();
        login_screen.draw(draw_area);
    });
}

function get_channel_lists(on_complete, on_fail) {
    $.ajax( "/api/v1/get_channel_lists", {
        "method": "GET",
    }).done( function(data_str) {
        on_complete(JSON.parse(data_str));
    }).fail( function() {
        on_fail()
    });
}

function get_channel_data(channel_name, on_complete, on_fail) {
    $.ajax( "/api/v1/get_channel_list?list_name="+channel_name, {
        "method": "GET",
    }).done( function(data_str) {
        on_complete(JSON.parse(data_str));
    }).fail( function() {
        on_fail()
    });
}

function draw_channel_list(draw_area, channel_edit_area) {
    let channel_list = $( "<ul></ul>" );

    get_channel_lists(
        // On success
        function( data ) {
            console.log(data);
            data.forEach(function (channel_name) {
                let channel = $( "<li></li>" ).text(channel_name);
                channel.click(function() {
                    draw_channel(channel_name, channel_edit_area);
                });
                channel_list.append(channel);
            });
        },
        // On fail
        function() {
            // TODO fix this
            alert("Failed to get the channel lists, please reload");
        });

    draw_area.empty();
    draw_area.append(channel_list);
}

function draw_channel(channel_name, draw_area) {
    let channel_edit_buttons = $( "<form>" +
        '<label for="new_sublist_name">New Sublist Name:</label>
        '<input type="textarea" id="new_sublist_name">'
        "</form>" );
    let channel_edit_list = $( "<ul></ul>" );

    get_channel_data(channel_name, 
        // On complete
        function( data ) {
            let entries = data.entries;
            entries.forEach(function (entry) {
                /*  Entry format:
                        name (str), image (str url), type (sublist or video)
                    With type sublist:
                        sublist (Entry type, so recursive)
                    With type video:
                        vidurl (str url), vidfmt (str formats - mp4, to start)
                */
                let channel_entry = $( "<li></li>" ).text(entry.name);
                channel_entry.click(function(entry) {
                    // TODO
                    console.log(entry);
                });
                channel_edit_list.append(channel_entry);
            });
        },
        // On fail
        function() {
            // TODO fix this
            alert("Failed to get the channel list, please reload");
        });

    let create_sublist_button = $( '<input type="button" value="Create Sublist">' )
        .click(function() {
            // TODO
            // create the sublist and add it to the common data store of the list
            // use the api call to store the new version of the list
            // re-draw the list
        });

    channel_edit_buttons.append(create_sublist_button);

    draw_area.empty();
    draw_area.append(channel_edit_list);
    draw_area.append(channel_edit_buttons);
}
