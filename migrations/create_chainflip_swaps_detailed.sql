-- Create new chainflip_swaps_detailed table
CREATE TABLE IF NOT EXISTS chainflip_swaps_detailed (
    id SERIAL PRIMARY KEY,
    timestamp INTEGER NOT NULL,
    date DATE NOT NULL,
    swap_id VARCHAR(255) NOT NULL UNIQUE,
    source_asset VARCHAR(255) NOT NULL,
    dest_asset VARCHAR(255) NOT NULL,
    base_asset_leg1 VARCHAR(255),
    base_asset_leg2 VARCHAR(255),
    ingress_amount DOUBLE PRECISION NOT NULL,
    ingress_value_usd DOUBLE PRECISION NOT NULL,
    input_amount DOUBLE PRECISION NOT NULL,
    input_value_usd DOUBLE PRECISION NOT NULL,
    output_amount DOUBLE PRECISION NOT NULL,
    output_value_usd DOUBLE PRECISION NOT NULL,
    started_block_date VARCHAR(255),
    started_block_id BIGINT,
    started_block_timestamp VARCHAR(255),
    destination_address VARCHAR(255) NOT NULL,
    refund_address VARCHAR(255),
    status VARCHAR(255) NOT NULL,
    broker VARCHAR(255),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes
CREATE INDEX IF NOT EXISTS chainflip_swaps_detailed_date_idx ON chainflip_swaps_detailed (date);
CREATE INDEX IF NOT EXISTS chainflip_swaps_detailed_source_asset_idx ON chainflip_swaps_detailed (source_asset);
CREATE INDEX IF NOT EXISTS chainflip_swaps_detailed_dest_asset_idx ON chainflip_swaps_detailed (dest_asset);
CREATE INDEX IF NOT EXISTS chainflip_swaps_detailed_status_idx ON chainflip_swaps_detailed (status); 