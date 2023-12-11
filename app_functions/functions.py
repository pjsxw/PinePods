import feedparser
import pprint
import requests


def search_podcast(e):
    if not search_pods.value:
        search_pods.error_text = "Please enter a podcast to seach for"
        page.update()
    else:
        podcast_value = search_pods.value
        page.clean()
        page.add(ft.Text(f"Searching for {podcast_value}!"))
        search_results = InternalFunctions.searchpod.searchpod(podcast_value)
        return_results = search_results['feeds']
        page.clean()
        # Allow scrolling otherwise the page will overflow
        page.scroll = "auto"
        page.update()

        # Create back button
        back_button = ft.IconButton(
            icon=ft.icons.ARROW_BACK_IOS_NEW_ROUNDED,
            icon_color='blue400',
            icon_size=30,
            tooltip='Return to Homepage',
            on_click=return_home,
            data=True
        )
        page.add(back_button)
        # cycle through podcasts and add results to page
        pod_number = 1

        for d in return_results:
            # print(d['title'])
            for k, v in d.items():
                if k == 'title':
                    # Defining the attributes of each podcast that will be displayed on screen
                    pod_image = ft.Image(src=d['image'], width=150, height=150)
                    pod_title = ft.TextButton(
                        text=d['title'],
                        on_click=evaluate_podcast
                    )
                    pod_desc = ft.Text(d['description'], no_wrap=False)
                    # Episode Count and subtitle
                    pod_ep_title = ft.Text('Episode Count:', weight=ft.FontWeight.BOLD)
                    pod_ep_count = ft.Text(d['episodeCount'])
                    pod_ep_info = ft.Row(controls=[pod_ep_title, pod_ep_count])
                    # Creating column and row for search layout
                    search_column = ft.Column(
                        wrap=True,
                        controls=[pod_title, pod_desc, pod_ep_info]
                    )
                    search_row = ft.Row(
                        wrap=True,
                        alignment=ft.MainAxisAlignment.START,
                        controls=[pod_image, search_column])

                    page.add(search_row)
                    pod_number += 1


def parse_feed(feed_url):
    d = feedparser.parse(feed_url)
    return d


def send_email(server_name, server_port, from_email, to_email, send_mode, encryption, auth_required, username, password,
               subject, body):
    import smtplib
    from email.mime.multipart import MIMEMultipart
    from email.mime.text import MIMEText
    import ssl
    import socket

    try:
        if send_mode == "SMTP":
            # Set up the SMTP server.
            if encryption == "SSL/TLS":
                smtp = smtplib.SMTP_SSL(server_name, server_port, timeout=10)
            elif encryption == "STARTTLS":
                smtp = smtplib.SMTP(server_name, server_port, timeout=10)
                smtp.starttls()
            else:  # No encryption
                smtp = smtplib.SMTP(server_name, server_port, timeout=10)

            # Authenticate if needed.
            if auth_required:
                try:  # Trying to login and catching specific SMTPNotSupportedError
                    smtp.login(username, password)
                except smtplib.SMTPNotSupportedError:
                    return 'SMTP AUTH extension not supported by server.'

            # Create a message.
            msg = MIMEMultipart()
            msg['From'] = from_email
            msg['To'] = to_email
            msg['Subject'] = subject
            msg.attach(MIMEText(body, 'plain'))

            # Send the message.
            smtp.send_message(msg)
            smtp.quit()
            return 'Email sent successfully.'

        elif send_mode == "Sendmail":
            pass
    except ssl.SSLError:
        return 'SSL Wrong Version Number. Try another ssl type?'
    except smtplib.SMTPAuthenticationError:
        return 'Authentication Error: Invalid username or password.'
    except smtplib.SMTPRecipientsRefused:
        return 'Recipients Refused: Email address is not accepted by the server.'
    except smtplib.SMTPSenderRefused:
        return 'Sender Refused: Sender address is not accepted by the server.'
    except smtplib.SMTPDataError:
        return 'Unexpected server response: Possibly the message data was rejected by the server.'
    except socket.gaierror:
        return 'Server Not Found: Please check your server settings.'
    except ConnectionRefusedError:
        return 'Connection Refused: The server refused the connection.'
    except TimeoutError:
        return 'Timeout Error: The connection to the server timed out.'
    except smtplib.SMTPException as e:
        return f'Failed to send email: {str(e)}'


def sync_with_nextcloud(nextcloud_url, nextcloud_token):
    print("Starting Nextcloud Sync")

    headers = {
        "Authorization": f"Bearer {access_token}",
        "Content-Type": "application/json"
    }

    # Sync Subscriptions
    sync_subscriptions(nextcloud_url, headers)

    # Sync Episode Actions
    sync_episode_actions(nextcloud_url, headers)


def sync_subscriptions(nextcloud_url, headers, user_id):
    # Implement fetching and updating subscriptions
    # Example GET request to fetch subscriptions
    response = requests.get(f"{nextcloud_url}/index.php/apps/gpoddersync/subscriptions", headers=headers)
    # Handle the response
    print(response.json())


def sync_subscription_change(nextcloud_url, headers, add, remove):
    payload = {
        "add": add,
        "remove": remove
    }
    response = requests.post(f"{nextcloud_url}/index.php/apps/gpoddersync/subscription_change/create", json=payload,
                             headers=headers)
    # Handle the response


def sync_episode_actions(nextcloud_url, headers):
    print('test')
    # Implement fetching and creating episode actions
    # Similar to the sync_subscriptions method


if __name__ == "__main__":
    # Example usage
    feed_url = "https://feeds.npr.org/510318/podcast.xml"
    d = parse_feed(feed_url)
    for entry in d.entries:
        audio_file = None
        for link in entry.links:
            if link.get("type", "").startswith("audio/"):
                audio_file = link.href
                break
        if audio_file:
            print("\n")
            print("Title: ", entry.title)
            print("Link: ", entry.link)
            print("Published Date: ", entry.published)
            content = entry.get('content', [{}])[0].get('value', entry.description)
            print("Content/Description: ", content)
            print("Audio File: ", audio_file)
            parsed_artwork_url = entry.get('itunes_image', {}).get('href', None) or entry.get('image', {}).get('href',
                                                                                                               None)
            print(parsed_artwork_url)
        else:
            print("\n")
            print("Title: ", entry.title)
            print("Link: ", entry.link)
            print("Description: ", entry.description)
            print("No audio file found for this entry")
            print("Published Date: ", entry.published)
            print(entry.itunes_image)
