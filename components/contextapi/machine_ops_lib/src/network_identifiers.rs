// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use db_interaction::models::networks::NetworkIdentifier;
use diesel::prelude::*;

/// Upserts the given network identifiers into the database.
pub fn upsert_network_ids(
    network_ids: &[i16],
    conn: &mut SqliteConnection,
) -> Result<(), diesel::result::Error> {
    conn.transaction(|conn| {
        for network_id in network_ids {
            let network_identifier = NetworkIdentifier { id: *network_id };

            network_identifier
                .insert_into(db_interaction::schema::network_identifiers::table)
                .on_conflict_do_nothing()
                .execute(conn)?;
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use db_interaction::models::context_id::ContextIdBytes;
    use db_interaction::models::contexts::ContextIdentifier;
    use db_interaction::models::networks::Network;
    use db_interaction::schema::network_identifiers as network_identifiers_schema;
    use db_interaction::schema::networks as networks_schema;
    use db_interaction::test_utils::TestDb;
    use uuid::Uuid;

    // Load the network identifiers from the db and assert that they generate the same
    // set as the given `expected_ids`.
    fn assert_db_network_identifiers(
        expected_ids: impl IntoIterator<Item = i16>,
        conn: &mut SqliteConnection,
    ) {
        let net_ids_in_db: Vec<i16> = network_identifiers_schema::table
            .select(network_identifiers_schema::id)
            .load(conn)
            .unwrap();

        assert_eq!(
            net_ids_in_db.into_iter().collect::<HashSet<i16>>(),
            expected_ids.into_iter().collect::<HashSet<i16>>()
        );
    }

    #[test]
    fn upsert_net_ids_empty_db_works() {
        let net_ids = vec![1, 2, 3];
        let mut db = TestDb::spawn();
        assert!(upsert_network_ids(&net_ids, &mut db.conn).is_ok());
        assert_db_network_identifiers(net_ids, &mut db.conn);
    }

    #[test]
    fn upsert_net_ids_is_idempotent() {
        let net_ids = vec![1, 2, 3];
        let mut db = TestDb::spawn();
        // first run
        assert!(upsert_network_ids(&net_ids, &mut db.conn).is_ok());
        assert_db_network_identifiers(net_ids.clone(), &mut db.conn);

        // second run
        assert!(upsert_network_ids(&net_ids, &mut db.conn).is_ok());
        assert_db_network_identifiers(net_ids, &mut db.conn);
    }

    // Check that upsert does not overwrite existing network ids or
    // affect networks depending on them.
    #[test]
    fn upsert_does_not_affect_existing_network_ids() {
        let overlapping_net_id = 3;
        let net_ids1 = vec![1, 2, overlapping_net_id];
        let net_ids2 = vec![overlapping_net_id, 4, 5];

        let mut db = TestDb::spawn();

        // upsert net_ids1
        assert!(upsert_network_ids(&net_ids1, &mut db.conn).is_ok());

        // Insert a network depending on the overlapping network id and check that it
        // still exists after inserting `net_ids2` (which also contains the overlapping network id).

        let context = ContextIdentifier {
            id: ContextIdBytes::from(Uuid::new_v4()),
        };

        context
            .insert_into(db_interaction::schema::contexts::table)
            .execute(&mut db.conn)
            .unwrap();

        let network = Network {
            id: overlapping_net_id,
            name: String::from("foo"),
            context_id: context.id,
        };

        network
            .clone()
            .insert_into(networks_schema::table)
            .execute(&mut db.conn)
            .unwrap();
        // upsert net_ids2
        assert!(upsert_network_ids(&net_ids2, &mut db.conn).is_ok());

        // Check that `network` is still in the database
        let loaded_network: Network = networks_schema::table
            .select(Network::as_select())
            .find(overlapping_net_id)
            .first(&mut db.conn)
            .unwrap();

        assert_eq!(network, loaded_network);

        // check that the union of the upserted network ids correspond to the network ids in the database
        assert_db_network_identifiers(net_ids1.into_iter().chain(net_ids2), &mut db.conn);
    }
}
