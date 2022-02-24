FROM atmoz/sftp:latest

ADD ./.ssh/ssh_host_ed25519_key /etc/ssh/
ADD ./.ssh/ssh_host_rsa_key /etc/ssh/

RUN chmod 600 /etc/.ssh/ssh_host_*
