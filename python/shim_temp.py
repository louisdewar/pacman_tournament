import socket
import json
import sys
import select

class Ai:
    def __init__(self, address, port, username, code):
        print('Connecting to {}:{}'.format(address, port))
        self.sock = socket.create_connection((address, port))
        self.sock.send(bytes(json.dumps({ 'username': username, 'code': code }) + '\n', 'utf-8'))

    def start(self):
        buf = ''

        while True:
            index = buf.find('\n')
            if index != -1:
                msg = buf[0:index]
                buf = buf[index + 1:]
                try:
                    data = json.loads(msg)
                    if 'error' in data:
                        print('There was an error:', data['error'])
                        return

                    if 'died' in data:
                        print('You died your final score was', data['died']['final_score'])
                        return

                    if 'spawned' in data:
                        print('You have been spawned into game id', data['spawned']['game_id'])
                        continue

                    if 'tick' in data:
                        tick = data['tick']
                        action = self.choose_action(tick['view'])
                        self.sock.send(bytes(json.dumps({ 'tick': tick['tick'], 'action': action }) + '\n', 'utf-8'))
                except json.decoder.JSONDecodeError:
                    print('msg `%s` from the server was invalid json' % msg)
            else:
                try:
                    ready_to_read, _write, in_error = select.select([self.sock], [], [self.sock])
                except select.error:
                    print('Network error')
                    self.sock.shutdown(2)
                    self.sock.close()
                    return

                if len(ready_to_read) > 0:
                    incoming = self.sock.recv(4096).decode('utf-8')
                    if len(incoming) == 0:
                        print('Network error (ready 0 bytes after socket was ready)')
                        self.sock.shutdown(2)
                        self.sock.close()
                        return

                    buf += incoming

    def choose_action(self, game):
        """ 
            You're code goes here, you must return an action:
            'F' => Forward
            'R' => TurnRight
            'L' => TurnLeft
            'E' => Eat
            'S' => Stay
        """
        current_player = game[1][2]['player']
        current_direction = current_player['direction']

        reverse_map = {
            'N': 'S',
            'E': 'W',
            'S': 'N',
            'W': 'E',
        }

        reverse = reverse_map[current_direction]

        if game[1][1]['base'] != 'L':
            return 'R'

        enemy_player = game[1][1]['player']
        if enemy_player and enemy_player['direction'] == reverse:
            return 'F'

        if not game[1][1]['player']:
            return 'F';

        return 'S'


if __name__ =='__main__':
    try:
        ip = sys.argv[1]
        port = sys.argv[2]
        userid = int(sys.argv[3])
    except IndexError:
        print("Usage: python3 shim.py [ip] [port] [user id]")
        sys.exit(1)

    f = open("creds", "r")

    users = f.read().split('\n')
    user = users[userid - 1].split(' ')
    username = user[0]
    code = user[1]

    ai = Ai(ip, port, username, code)

    ai.start()
